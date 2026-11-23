using System.Runtime.InteropServices.JavaScript;
using Cantrip.Server.Rules;

namespace Cantrip.Client.Web;

/// <summary>
/// The rules engine in this page (Engine.js; docs/engine-in-the-browser.md): <c>crates/cantrip-wasm</c>, answering the
/// native host's requests (docs/host-protocol.md) as <c>cantrip-native-host</c> answers them on a server, reply for reply.
/// A game played against it is a server's table in this page: <see cref="NativeRulesHost"/> over
/// <see cref="BrowserEngineTransport"/>, behind a <c>GameSession</c>, every seat a loopback transport.
/// </summary>
public static partial class BrowserEngine
{
    /// <summary>Where the module is, beside the page unless the address says otherwise (<c>?engine=</c>).</summary>
    public const string DefaultUrl = "engine/cantrip_wasm.wasm";

    [JSImport("globalThis.cantripEngineLoad")]
    private static partial Task Load(string url);

    [JSImport("globalThis.cantripEngineCall")]
    internal static partial string Call(string request);

    /// <summary>Fetches and starts the engine, once; a second call waits on the first.</summary>
    public static Task LoadAsync(string url) => Load(url);
}

/// <summary>The engine in this page as the host process a <see cref="NativeRulesHost"/> talks to.</summary>
public sealed class BrowserEngineTransport : IHostTransport
{
    public string Exchange(string request) => BrowserEngine.Call(request);
}
