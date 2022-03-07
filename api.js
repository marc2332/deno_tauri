export default class Window {
    id = Math.random().toString();
    title;
    url;
    html;
    constructor(title){
        this.title = title;
    }

    setUrl(url){
        this.url = url;
    }

    setHtml(html){
        this.html = html;
    }

    run(){
        return Deno.core.opAsync("runWindow", {
            id: this.id,
            title: this.title,
            content: this.url != null ? {
                _type: "Url",
                url: this.url
            }: {
                _type: "Html",
                html: this.html
            }
        });
    }

    send(event, content){
        return Deno.core.opAsync("sendToWindow", {
            id: this.id,
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
