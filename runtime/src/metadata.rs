use deno_graph::ModuleSpecifier;
use serde::{Deserialize, Serialize};

// Inspired by https://github.com/denoland/deno/blob/8b2989c417db9090913f1cb6074ae961f4c14d5e/cli/standalone.rs#L46
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub entrypoint: ModuleSpecifier,
    pub author: String,
    pub name: String
}
