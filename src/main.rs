#![feature(map_try_insert)]

use custom_extension::RunWindowMessage;
use custom_extension::SentToWindowMessage;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::located_script_name;
use deno_core::v8_set_flags;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::permissions::Permissions;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::BootstrapOptions;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncSeekExt;
use tokio::macros::support::Pin;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use deno_core::futures::FutureExt;
use std::collections::HashMap;
use std::env::current_exe;
use std::io::SeekFrom;
use std::iter::once;
use std::rc::Rc;
use std::sync::Arc;
use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::{Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

mod custom_extension;

fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

#[derive(Debug)]
pub enum AstrodonMessage {
    SentToWindowMessage(SentToWindowMessage),
    RunWindowMessage(RunWindowMessage),
    SentToDenoMessage(String, String),
}

#[derive(Debug)]
enum WryEvent {
    RunScript(String, String),
    NewWindow(RunWindowMessage),
}

struct EmbeddedModuleLoader(eszip::EszipV2);

impl ModuleLoader for EmbeddedModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        base: &str,
        _is_main: bool,
    ) -> Result<ModuleSpecifier, AnyError> {
        let resolve = deno_core::resolve_import(specifier, base)?;
        Ok(resolve)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dynamic: bool,
    ) -> Pin<Box<deno_core::ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();

        let module = self
            .0
            .get_module(module_specifier.as_str())
            .ok_or_else(|| type_error("Module not found"));

        async move {
            let module = module?;

            let code = module.source().await;

            let code = std::str::from_utf8(&code)
                .map_err(|_| type_error("Module source is not utf-8"))?
                .to_owned();

            Ok(deno_core::ModuleSource {
                code,
                module_type: match module.kind {
                    eszip::ModuleKind::JavaScript => deno_core::ModuleType::JavaScript,
                    eszip::ModuleKind::Json => deno_core::ModuleType::Json,
                },
                module_url_specified: module_specifier.to_string(),
                module_url_found: module_specifier.to_string(),
            })
        }
        .boxed_local()
    }
}

#[tokio::main]
async fn main() {
    let (snd, mut rev) = mpsc::unbounded_channel::<AstrodonMessage>();
    let subs = Arc::new(Mutex::new(HashMap::new()));

    let deno_sender = snd.clone();
    let deno_subs = subs.clone();

    std::thread::spawn(move || {
        let r = tokio::runtime::Runtime::new().unwrap();

        // Kinda ugly to run a whole separated tokio runtime just for deno, might improve this eventually
        r.block_on(async move {
            let eszip = extract_standalone().await.unwrap().unwrap();

            let module_loader = Rc::new(EmbeddedModuleLoader(eszip));
            let create_web_worker_cb = Arc::new(|_| {
                todo!("Web workers are not supported in the example");
            });

            v8_set_flags(
                once("UNUSED_BUT_NECESSARY_ARG0".to_owned())
                    .chain(Vec::new().iter().cloned())
                    .collect::<Vec<_>>(),
            );

            let options = WorkerOptions {
                bootstrap: BootstrapOptions {
                    apply_source_maps: false,
                    args: vec![],
                    cpu_count: 1,
                    debug_flag: false,
                    enable_testing_features: false,
                    location: None,
                    no_color: false,
                    runtime_version: "0".to_string(),
                    ts_version: "0".to_string(),
                    unstable: false,
                },
                extensions: vec![custom_extension::new(deno_sender, deno_subs.clone())],
                unsafely_ignore_certificate_errors: None,
                root_cert_store: None,
                user_agent: "hello_runtime".to_string(),
                seed: None,
                js_error_create_fn: None,
                create_web_worker_cb,
                maybe_inspector_server: None,
                should_break_on_first_statement: false,
                module_loader,
                get_error_class_fn: Some(&get_error_class_name),
                origin_storage_dir: None,
                blob_store: BlobStore::default(),
                broadcast_channel: InMemoryBroadcastChannel::default(),
                shared_array_buffer_store: None,
                compiled_wasm_module_store: None,
            };

            let mut cwd = std::env::current_dir().unwrap();
            // remove '/compile'
            cwd.pop();

            let cwd = cwd.as_path().display().to_string();

            let main_module =
                ModuleSpecifier::from(format!("file:///{}/test.js", cwd).parse().unwrap());

            let permissions = Permissions::allow_all();

            let mut worker =
                MainWorker::bootstrap_from_options(main_module.clone(), permissions, options);

            worker.js_runtime.sync_ops_cache();

            worker
                .execute_main_module(&main_module)
                .await
                .expect("Could not run the application.");

            worker.dispatch_load_event(&located_script_name!()).unwrap();

            worker
                .run_event_loop(true)
                .await
                .expect("Could not run the application.");

            worker.dispatch_load_event(&located_script_name!()).unwrap();

            std::process::exit(0);
        });
    });

    let event_loop = EventLoop::<WryEvent>::with_user_event();
    let mut webviews: HashMap<WindowId, WebView> = HashMap::new();
    let mut custom_id_mapper: HashMap<String, WindowId> = HashMap::new();

    let proxy = event_loop.create_proxy();
    let l_proxy = proxy.clone();

    // custom event loop - this basically process and forwards events to the wry event loop
    tokio::task::spawn(async move {
        loop {
            match rev.recv().await.unwrap() {
                AstrodonMessage::SentToWindowMessage(msg) => {
                    l_proxy.send_event(WryEvent::RunScript(
                        msg.id,
                        format!(
                            "window.dispatchEvent(new CustomEvent('{}', {{detail: JSON.parse({})}}));",
                            msg.event, msg.content
                        ),
                    )).expect("Could not dispatch event");
                }
                AstrodonMessage::RunWindowMessage(msg) => {
                    l_proxy
                        .send_event(WryEvent::NewWindow(msg))
                        .expect("Could not open a new window");
                }
                AstrodonMessage::SentToDenoMessage(name, content) => {
                    let events = subs.lock().await;
                    let subs = events.get(&name);
                    if let Some(subs) = subs {
                        for sub in subs.values() {
                            sub.send(content.clone()).unwrap();
                        }
                    }
                }
            }
        }
    });

    // Run the wry event loop
    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => {
                    webviews.remove(&window_id);
                    custom_id_mapper.retain(|_, v| *v != window_id);

                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit
                    }
                }
                WindowEvent::Resized(_) => {
                    let _ = webviews[&window_id].resize();
                }
                _ => (),
            },
            Event::UserEvent(WryEvent::RunScript(window_id, content)) => {
                let id = custom_id_mapper.get(&window_id);
                if let Some(id) = id {
                    webviews
                        .get(&id)
                        .unwrap()
                        .evaluate_script(&content)
                        .expect("Could not run the script");
                }
            }
            Event::UserEvent(WryEvent::NewWindow(msg)) => {
                let new_window = create_new_window(msg.title, msg.url, &event_loop, snd.clone());
                custom_id_mapper.insert(msg.id, new_window.0.clone());
                webviews.insert(new_window.0, new_window.1);
            }
            _ => (),
        }
    });
}

#[derive(Serialize, Deserialize)]
struct SendEvent {
    name: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum IpcMessage {
    SendEvent { name: String, content: String },
}

fn create_new_window(
    title: String,
    url: String,
    event_loop: &EventLoopWindowTarget<WryEvent>,
    snd: UnboundedSender<AstrodonMessage>,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .build(event_loop)
        .unwrap();

    let window_id = window.id();

    let handler = move |_: &Window, req: String| {
        let message: IpcMessage = serde_json::from_str(&req).unwrap();

        match message {
            IpcMessage::SendEvent { name, content } => {
                snd.send(AstrodonMessage::SentToDenoMessage(name, content))
                    .unwrap();
            }
        }
    };

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_url(&url)
        .unwrap()
        .with_initialization_script("
        globalThis.sendToDeno = (name, content) => {
            window.ipc.postMessage(JSON.stringify({type:'SendEvent', name, content: JSON.stringify(content) }));
        }
         ")
        .with_ipc_handler(handler)
        .with_dev_tool(true)
        .build()
        .unwrap();

    (window_id, webview)
}

fn u64_from_bytes(arr: &[u8]) -> Result<u64, AnyError> {
    use deno_core::anyhow::Context;
    let fixed_arr: &[u8; 8] = arr
        .try_into()
        .context("Failed to convert the buffer into a fixed-size array")?;
    Ok(u64::from_be_bytes(*fixed_arr))
}

pub const MAGIC_TRAILER: &[u8; 8] = b"4str0d0n";

pub async fn extract_standalone() -> Result<Option<eszip::EszipV2>, AnyError> {
    use deno_core::anyhow::Context;
    use tokio::io::AsyncReadExt;
    let current_exe_path = current_exe()?;

    let file = tokio::fs::File::open(&current_exe_path).await?;

    let mut bufreader = tokio::io::BufReader::new(file);

    bufreader.seek(SeekFrom::End(-16)).await?;

    let mut trailer = [0; 16];

    bufreader.read_exact(&mut trailer).await?;

    let (magic_trailer, eszip_archive_pos) = trailer.split_at(8);

    if magic_trailer != MAGIC_TRAILER {
        return Ok(None);
    }

    let eszip_archive_pos = u64_from_bytes(eszip_archive_pos)?;

    bufreader.seek(SeekFrom::Start(eszip_archive_pos)).await?;

    let (eszip, loader) = eszip::EszipV2::parse(bufreader)
        .await
        .context("Failed to parse eszip header")?;

    loader.await.context("Failed to parse eszip archive")?;

    Ok(Some(eszip))
}
