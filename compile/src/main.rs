use std::collections::HashMap;
use std::sync::Arc;

use deno_ast::EmitOptions;
use deno_core::ModuleSpecifier;
use deno_graph::source::ResolveResponse;
use deno_runtime::permissions::PermissionsOptions;
use log::Level;
use std::env;
use tokio::io::AsyncWriteExt;
use url::Url;

pub const MAGIC_TRAILER: &[u8; 8] = b"4str0d0n";

#[tokio::main]
async fn main() {
    let mut p = env::current_dir().unwrap();
    p.pop();
    let s = &format!("file://{}/test.js", p.to_str().unwrap());

    let url = Url::parse(s).unwrap();

    let graph = deno_graph::create_code_graph(
        vec![(url, deno_graph::ModuleKind::Esm)],
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

    let mut eszip = eszip::EszipV2::from_graph(graph, EmitOptions::default()).unwrap();

    let mut eszip_archive = eszip.into_bytes();

    let mut original_bin = tokio::fs::read("../target/debug/deno_wry.exe")
        .await
        .unwrap();

    let eszip_pos = original_bin.len();

    let mut trailer = MAGIC_TRAILER.to_vec();

    trailer.write_all(&eszip_pos.to_be_bytes()).await.unwrap();

    let mut final_bin =
        Vec::with_capacity(original_bin.len() + eszip_archive.len() + trailer.len());
    final_bin.append(&mut original_bin);
    final_bin.append(&mut eszip_archive);
    final_bin.append(&mut trailer);

    tokio::fs::write("./test.exe", final_bin).await.unwrap();
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
