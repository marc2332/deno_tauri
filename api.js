export default class Window {
    #id = Math.random().toString();
    #title;
    url;
    constructor(title, url){
        this.#title = title;
        this.url = url;
    }

    run(){
        Deno.core.opSync("runWindow", {
            id: this.#id,
            title: this.#title,
            url: this.url
        });
    }

    send(event, content){
        Deno.core.opSync("sendToWindow", {
            id: this.#id,
            event,
            content: JSON.stringify(content)
        });
    }

    listen = async function* (name) {
        while (true){
            yield await Deno.core.opAsync("listenEvent",{name});
        }
    }
}
