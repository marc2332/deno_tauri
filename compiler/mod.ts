import { build } from "https://raw.githubusercontent.com/denoland/eszip/main/lib/mod.ts";
import { writeAll } from "https://deno.land/std@0.128.0/streams/conversion.ts";

/**
 * Like `deno compile` but for our custom runtime
 * 
 * @param input TypeScript / JavaScript file
 * @param output Output dir
 */
export async function compile(input: string, output: string, author: string, name: string){
    
    const eszip = await build([input]);
    const original_bin = await Deno.readFile("./target/release/runtime.exe");
    const final_bin = await Deno.create(output);
    
    const eszip_pos = original_bin.length;
    const metadata_pos = eszip_pos + eszip.length;
    
    const trailer = new Uint8Array([
        ...new TextEncoder().encode("4str0d0n"), 
        ...numberToByteArray(eszip_pos), 
        ...numberToByteArray(metadata_pos)
    ]);
    
    const metadata = {
        entrypoint: input,
        author,
        name
    }
    
    await writeAll(final_bin, original_bin);
    await writeAll(final_bin, eszip);
    await writeAll(final_bin, new TextEncoder().encode(JSON.stringify(metadata)));
    await writeAll(final_bin, trailer);
    
    await final_bin.close()
    
}

const numberToByteArray = (x: number) => {
    const y= Math.floor(x/2**32);
    return [y,(y<<8),(y<<16),(y<<24), x,(x<<8),(x<<16),(x<<24)].map(z=> z>>>24)
}