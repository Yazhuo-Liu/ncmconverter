import init, { convert } from "./wasm/ncmc_wasm.js";

const wasmReady = init();

self.addEventListener("message", (event) => {
    void convertFile(event.data);
});

async function convertFile({ id, buffer }) {
    try {
        await wasmReady;
        self.postMessage({ type: "started", id });

        const result = convert(new Uint8Array(buffer));

        try {
            const audio = result.audio;
            const audioBuffer = audio.buffer;

            self.postMessage(
                {
                    type: "success",
                    id,
                    audioBuffer,
                    format: result.format,
                    extension: result.extension,
                    mimeType: result.mimeType,
                    title: result.title,
                    artist: result.artist,
                    album: result.album,
                },
                [audioBuffer],
            );
        } finally {
            result.free();
        }
    } catch (error) {
        self.postMessage({
            type: "error",
            id,
            message: readableError(error),
        });
    }
}

function readableError(error) {
    if (typeof error === "string" && error.trim()) {
        return error;
    }

    if (error instanceof Error && error.message) {
        return error.message;
    }

    return "";
}
