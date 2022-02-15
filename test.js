import Window from './api.js'

const windowA = new Window("Window A",`file://${Deno.cwd()}/index.html`);

windowA.run();

const windowB = new Window("Window B",`file://${Deno.cwd()}/index.html`);

windowB.run();

setInterval(() => {
    windowA.send("some_data", {number: Math.random()})
    windowB.send("some_data", {number: Math.random()})
}, 1)



