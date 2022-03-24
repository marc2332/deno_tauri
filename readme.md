**!!!!! IMPORTANT !!!!!**: This has been ported to [Astrodon under `deno_tauri` branch](https://github.com/astrodon/astrodon/tree/feature/deno_tauri)

This is extends the [Deno](https://deno.land/) runtime by adding some new features:

- Create webview windows using [wry](https://github.com/tauri-apps/wry)
- Bidirectional communication between the deno app and the windows

You can see the example located in `examples/demo.js`.

This is just a proof of concept

## Run

Compile the runtime project:
```
cargo build
```

Run the compiler over the demo app:
```
deno run -A .\compiler\cli.ts -i .\example\demo.js -a my_name -n my.super.app
```

Run the demo! 
```shell
./demo #.exe in Windows
```
