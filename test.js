import Window from './api.js'

const windowA = new Window("Window A",`file://${Deno.cwd()}/index.html`);

windowA.run();

for await (const msg of await windowA.listen("to-deno")){
    windowA.send("from-deno", msg);
}
