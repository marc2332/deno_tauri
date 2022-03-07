**warning**: you might see ugly code

This is extends the [Deno](https://deno.land/) runtime by adding some new features:

- Create webview windows using [wry](https://github.com/tauri-apps/wry)
- Bidirectional communication between the deno app and the windows

- Deno app: test.js
- Webview app: index.html


This is just a poc atm, it will be eventually ported to https://github.com/astrodon/astrodon


## Issue with eszip

I want to implement something like `deno compile` into this, but, I can't get eszip to read the appended file.

The compile implementation is under `./compile`, it reads the test.js file and appends it to the binary created on the root project.

Steps to reproducte:

Compile the root project:
```
cargo build
```

Run the compiler:
```
cd compile
cargo run
```

The output binary is `./compile/test`

The issue is that it gets stucked when trying to retrieve the module (`module.source().await;`).
I think compile implementation is fine, or at least, partially, because the MAGIC_TRAILER is correctly found (as it should).
I avoided implementing the metadata thing, I thought it would be easier but maybe it's missing something now because of it 🤔 

