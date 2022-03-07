use common::Metadata;
use deno_ast::EmitOptions;
use deno_core::serde_json;
use deno_graph::source::ResolveResponse;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use url::Url;

pub const MAGIC_TRAILER: &[u8; 8] = b"4str0d0n";

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let cwd = env::current_dir().unwrap();
    let bin = env::current_exe().unwrap();

    // Skip the CLI's bin
    args.next();

    if let Some(entrypoint) = args.next() {
        let filename = Path::new(&entrypoint).file_name().unwrap();

        let mut final_bin_path = cwd.join(filename);
        final_bin_path.set_extension(bin.extension().unwrap());

        let entrypoint = cwd.join(entrypoint);

        let entrypoint = &format!("file://{}", entrypoint.to_str().unwrap());

        let entrypoint = Url::parse(entrypoint).unwrap();

        let graph = deno_graph::create_code_graph(
            vec![(entrypoint.clone(), deno_graph::ModuleKind::Esm)],
            false,
            None,
            &mut Loader,
            Some(&Resolver),
            None,
            None,
            None,
        )
        .await;

        graph.valid().unwrap();

        let eszip = eszip::EszipV2::from_graph(graph, EmitOptions::default()).unwrap();

        let mut eszip_archive = eszip.into_bytes();

        let mut original_bin = tokio::fs::read("../target/debug/runtime.exe")
            .await
            .unwrap();

        let eszip_pos = original_bin.len();

        let metadata = Metadata { entrypoint };
        let mut metadata = serde_json::to_string(&metadata)
            .unwrap()
            .as_bytes()
            .to_vec();
        let metadata_pos = eszip_pos + eszip_archive.len();

        let mut trailer = MAGIC_TRAILER.to_vec();

        trailer.write_all(&eszip_pos.to_be_bytes()).await.unwrap();
        trailer
            .write_all(&metadata_pos.to_be_bytes())
            .await
            .unwrap();

        let mut final_bin =
            Vec::with_capacity(original_bin.len() + eszip_archive.len() + trailer.len());
        final_bin.append(&mut original_bin);
        final_bin.append(&mut eszip_archive);
        final_bin.append(&mut metadata);
        final_bin.append(&mut trailer);

        tokio::fs::write(final_bin_path, final_bin).await.unwrap();
    } else {
        println!("Entrypoint file was not specified");
    }
}

#[derive(Debug)]
struct Resolver;

impl deno_graph::source::Resolver for Resolver {
    fn resolve(&self, specifier: &str, referrer: &deno_graph::ModuleSpecifier) -> ResolveResponse {
        match deno_graph::resolve_import(specifier, referrer) {
            Ok(specifier) => ResolveResponse::Specifier(specifier),
            Err(err) => ResolveResponse::Err(err.into()),
        }
    }
}

struct Loader;

impl deno_graph::source::Loader for Loader {
    fn load(
        &mut self,
        specifier: &deno_graph::ModuleSpecifier,
        is_dynamic: bool,
    ) -> deno_graph::source::LoadFuture {
        let specifier = specifier.clone();

        Box::pin(async move {
            if is_dynamic {
                return Ok(None);
            }

            match specifier.scheme() {
                "data" => deno_graph::source::load_data_url(&specifier),
                "file" => {
                    let path = tokio::fs::canonicalize(specifier.to_file_path().unwrap()).await?;
                    let content = tokio::fs::read(&path).await?;
                    let content = String::from_utf8(content)?;

                    Ok(Some(deno_graph::source::LoadResponse::Module {
                        specifier: Url::from_file_path(&path).unwrap(),
                        maybe_headers: None,
                        content: Arc::new(content),
                    }))
                }
                _ => Err(anyhow::anyhow!(
                    "unsupported scheme: {}",
                    specifier.scheme()
                )),
            }
        })
    }
}
