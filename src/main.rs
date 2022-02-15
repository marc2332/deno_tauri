use custom_extension::RunWindowMessage;
use custom_extension::SentToWindowMessage;
use deno_core::error::AnyError;
use deno_core::FsModuleLoader;
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::permissions::Permissions;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::BootstrapOptions;

use std::sync::mpsc;

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::{Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

mod custom_extension;

fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

pub enum AstrodonMessage {
    SentToWindowMessage(SentToWindowMessage),
    RunWindowMessage(RunWindowMessage),
}

#[derive(Debug)]
enum WryEvent {
    RunScript(String, String),
    CloseWindow(WindowId),
    NewWindow(RunWindowMessage),
}

#[tokio::main]
async fn main() {
    let (snd, rev) = mpsc::channel::<AstrodonMessage>();

    std::thread::spawn(move || {
        let r = tokio::runtime::Runtime::new().unwrap();

        let module_loader = Rc::new(FsModuleLoader);
        let create_web_worker_cb = Arc::new(|_| {
            todo!("Web workers are not supported in the example");
        });

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
            extensions: vec![custom_extension::new(snd)],
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

        let js_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test.js");
        let main_module =
            deno_core::resolve_path(&js_path.to_string_lossy()).expect("Could not find the app.");
        let permissions = Permissions::allow_all();

        let mut worker =
            MainWorker::bootstrap_from_options(main_module.clone(), permissions, options);

        worker.js_runtime.sync_ops_cache();

        r.block_on(worker.execute_main_module(&main_module))
            .expect("Could not run the application.");
        r.block_on(worker.run_event_loop(false))
            .expect("Could not run the application.");
    });

    let event_loop = EventLoop::<WryEvent>::with_user_event();
    let mut webviews: HashMap<WindowId, WebView> = HashMap::new();
    let mut custom_id_mapper: HashMap<String, WindowId> = HashMap::new();

    let proxy = event_loop.create_proxy();
    let l_proxy = proxy.clone();

    // custom event loop - this basically process and forwards events to the wry event loop
    tokio::task::spawn(async move {
        loop {
            match rev.recv().unwrap() {
                AstrodonMessage::SentToWindowMessage(msg) => {
                    l_proxy.send_event(WryEvent::RunScript(
                        msg.id,
                        format!(
                            "window.dispatchEvent(new CustomEvent('{}', {{detail: {}}}));",
                            msg.event, msg.content
                        ),
                    )).expect("Could not dispatch event");
                }
                AstrodonMessage::RunWindowMessage(msg) => {
                    l_proxy.send_event(WryEvent::NewWindow(msg)).expect("Could not open a new window");
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
                let id = custom_id_mapper.get(&window_id).unwrap();
                webviews.get(&id).unwrap().evaluate_script(&content).expect("Could not run the script");
            }
            Event::UserEvent(WryEvent::NewWindow(msg)) => {
                let new_window = create_new_window(msg.title, msg.url, &event_loop, proxy.clone());
                custom_id_mapper.insert(msg.id, new_window.0.clone());
                webviews.insert(new_window.0, new_window.1);
            }
            Event::UserEvent(WryEvent::CloseWindow(id)) => {
                webviews.remove(&id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            _ => (),
        }
    });
}

fn create_new_window(
    title: String,
    url: String,
    event_loop: &EventLoopWindowTarget<WryEvent>,
    proxy: EventLoopProxy<WryEvent>,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .build(event_loop)
        .unwrap();
        
    let window_id = window.id();

    let handler = move |window: &Window, req: String| match req.as_str() {
        "close" => {
            let _ = proxy.send_event(WryEvent::CloseWindow(window.id()));
        }
        _ => {}
    };

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_url(&url)
        .unwrap()
        .with_ipc_handler(handler)
        .with_dev_tool(true)
        .build()
        .unwrap();
    (window_id, webview)
}
