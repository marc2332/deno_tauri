import { parse , resolve } from "https://deno.land/std@0.128.0/path/mod.ts";
import { compile } from './mod.ts'
import { Command } from "https://deno.land/x/cliffy/command/mod.ts";

const { options } = await new Command()
  .option("-i, --input [type:string]", "Input path.")
  .option("-o, --output [type:string]", "Output path.")
  .option("-a, --author [type:string]", "Author name.")
  .option("-n, --name [type:string]", "App name.")
  .parse(Deno.args)

const input = new URL(`file://${resolve( Deno.cwd(), options.input)}`).href;

const output =  options.output ? resolve(Deno.cwd(),options.output) : resolve(Deno.cwd(), `${parse(options.input).name}${Deno.build.os === "windows" ? ".exe" : ""}`);

await compile(input, output, options.author, options.name);