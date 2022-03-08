import { parse , resolve } from "https://deno.land/std@0.128.0/path/mod.ts";
import { compile } from './mod.ts'

const entrypoint = Deno.args[1];

if(entrypoint == null) {
    console.log("Entrypoint file was not specified")
    Deno.exit(1)
}

const input = new URL(`file://${resolve( Deno.cwd(), entrypoint)}`).href;

const output =  Deno.args[2] ? resolve(Deno.cwd(), Deno.args[2]) : resolve(Deno.cwd(), `${parse(input).name}${Deno.build.os === "windows" ? ".exe" : ""}`);

await compile(input, output);