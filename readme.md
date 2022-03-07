**warning**: you might see ugly code

This is extends the [Deno](https://deno.land/) runtime by adding some new features:

- Create webview windows using [wry](https://github.com/tauri-apps/wry)
- Bidirectional communication between the deno app and the windows

- Deno app: test.js
- Webview app: index.html

This is just a poc atm, it will be eventually ported to https://github.com/astrodon/astrodon

## Run

Compile the root project:
```
cargo build
```

Run the compiler
```
cd compile
cargo run
```

Run the app! 
```
./compile/test
```

## to-do
- Integrate [Metadata](https://github.com/denoland/deno/blob/8b2989c417db9090913f1cb6074ae961f4c14d5e/cli/standalone.rs#L46)
- Improve the rusty code
- Move the `api.js` to TypeScript (this requires that it will need be transpiled)