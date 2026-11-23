// The rules engine in the page (?local; BrowserEngine.cs, docs/engine-in-the-browser.md): crates/cantrip-wasm built for
// the browser, a module that imports nothing. It answers the native host's JSON requests (docs/host-protocol.md), one
// request in and one reply out, synchronously, so the table that would talk to a server's engine talks to this one.
(function () {
    let engine = null;
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();

    // Fetches and instantiates the module once; later calls wait on the same load.
    let loading = null;
    globalThis.cantripEngineLoad = (url) => {
        loading ??= (async () => {
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`The rules engine could not be fetched from ${url} (${response.status}).`);
            }
            const { instance } = (response.headers.get("content-type") ?? "").includes("application/wasm")
                ? await WebAssembly.instantiateStreaming(response, {})
                : await WebAssembly.instantiate(await response.arrayBuffer(), {});
            engine = instance.exports;
        })();
        return loading;
    };

    globalThis.cantripEngineCall = (request) => {
        if (engine === null) {
            throw new Error("The rules engine is not loaded.");
        }
        const bytes = encoder.encode(request);
        const at = engine.cantrip_alloc(bytes.length);
        new Uint8Array(engine.memory.buffer, at, bytes.length).set(bytes);
        const length = engine.cantrip_call(at, bytes.length);
        engine.cantrip_free(at, bytes.length);
        return decoder.decode(new Uint8Array(engine.memory.buffer, engine.cantrip_reply(), length));
    };
})();
