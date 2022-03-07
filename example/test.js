const html = `
    <html>
        <body>
            <p>hi</p>
        </body>
        <script>
            window.addEventListener("from-deno", (ev) => {
                document.body.innerHTML = 'Data -> ' + ev.detail.some_number
            })

            setInterval(() => {
                window.sendToDeno("to-deno", { some_number: Math.random()});
            }, 1)
        </script>
    </html>
`;

const win = new AppWindow("Window A");

// win.setUrl("https://google.com") can also be used!

win.setHtml(html);

await win.run();

for await (const msg of await win.listen("to-deno")){
    win.send("from-deno", msg);
}
