import { readdir, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import assert from "node:assert/strict";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const webDirectory = join(scriptDirectory, "..");
const repositoryRoot = join(webDirectory, "..");
const wasmDirectory = join(webDirectory, "src", "wasm");
const fixtureDirectory = join(repositoryRoot, "crates", "ncmc", "tests", "input");

const wasmModule = await import(pathToFileURL(join(wasmDirectory, "ncmc_wasm.js")));
const wasmBytes = await readFile(join(wasmDirectory, "ncmc_wasm_bg.wasm"));
await wasmModule.default({ module_or_path: wasmBytes });

const fixtures = (await readdir(fixtureDirectory))
    .filter((name) => name.toLowerCase().endsWith(".ncm"))
    .sort();

assert.ok(fixtures.length > 0, "no NCM fixtures found");

const counts = new Map();

for (const fixture of fixtures) {
    const input = await readFile(join(fixtureDirectory, fixture));
    const result = wasmModule.convert(input);

    try {
        const audio = result.audio;
        assert.ok(audio.byteLength > 12, `${fixture}: output is unexpectedly short`);
        assert.equal(extensionFor(result.format), result.extension, `${fixture}: extension mismatch`);
        assert.equal(mimeFor(result.format), result.mimeType, `${fixture}: MIME mismatch`);
        assert.ok(hasExpectedHeader(audio, result.format), `${fixture}: invalid ${result.format} header`);

        counts.set(result.format, (counts.get(result.format) ?? 0) + 1);
        process.stdout.write(
            `verified ${fixture}: ${result.format}, ${audio.byteLength} bytes, ${result.title || "no title"}\n`,
        );
    } finally {
        result.free();
    }
}

let invalidError = "";
try {
    wasmModule.convert(new Uint8Array([0x00, 0x01, 0x02]));
} catch (error) {
    invalidError = String(error);
}
assert.match(
    invalidError,
    /could not decode the NCM file/i,
    "invalid input did not return a readable decode error",
);

const summary = [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([format, count]) => `${format}=${count}`)
    .join(", ");

process.stdout.write(`verified ${fixtures.length} browser-WASM conversions (${summary})\n`);

function extensionFor(format) {
    return {
        MP3: "mp3",
        FLAC: "flac",
        M4A: "m4a",
        OGG: "ogg",
    }[format];
}

function mimeFor(format) {
    return {
        MP3: "audio/mpeg",
        FLAC: "audio/flac",
        M4A: "audio/mp4",
        OGG: "audio/ogg",
    }[format];
}

function hasExpectedHeader(bytes, format) {
    const ascii = (start, end) => new TextDecoder("ascii").decode(bytes.subarray(start, end));

    if (format === "FLAC") return ascii(0, 4) === "fLaC";
    if (format === "OGG") return ascii(0, 4) === "OggS";
    if (format === "M4A") return ascii(4, 8) === "ftyp";
    if (format === "MP3") {
        return (
            ascii(0, 3) === "ID3" ||
            (bytes[0] === 0xff && (bytes[1] & 0xe0) === 0xe0 && (bytes[1] & 0x06) === 0x02)
        );
    }

    return false;
}
