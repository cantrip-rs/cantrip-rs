# The engine in the browser

The rules engine runs in a web page as well as beside a server: `crates/cantrip-wasm` is the engine built for
WebAssembly, answering the same JSON requests `cantrip-native-host` answers on its stdin
([host-protocol.md](host-protocol.md)). A game played on it needs no server. `?local` is such a game against the
tactical bot. A game that must be trusted is played the same way and replayed on a server's native engine, which answers
it identically.

## The module

`crates/cantrip-wasm` serves `cantrip_host::serve::Tables`, the requests and the games behind them, the same code the
stdio host serves (`crates/cantrip-host/src/bin/host.rs` keeps only its stdin loop and the AI recorder around it). The
module imports nothing. It exports its memory and four functions:

| Export | Does |
| --- | --- |
| `cantrip_alloc(len) -> ptr` | lends `len` bytes to write a request into |
| `cantrip_call(ptr, len) -> len` | answers the request there (one JSON object, UTF-8); returns the reply's length |
| `cantrip_reply() -> ptr` | where that reply is, until the next call |
| `cantrip_free(ptr, len)` | gives the request's bytes back |

Any page can instantiate it with `WebAssembly.instantiate(bytes, {})`. The web client's `Engine.js` does this, and
`crates/cantrip-wasm/check/determinism.mjs` shows the same few lines.

A fork's own definitions (a variant's emblems, say) join the engine's card table through
`cantrip_engine::add_cards`, called once before anything asks for the table. A fork's build of the module, and of the
native host it replays games on, registers them first, so both know them alike.

There is no unwinding on this target (the `wasm` profile aborts on a panic). Where the native host refuses a request the
engine failed on (`IllegalAnswer`), the module traps. The page then reports the failure, and the module is not used
again.

**Building it.** `tools\build-engine-wasm.cmd` builds the `wasm` profile (no debug information, LTO, optimised for size)
for `wasm32-unknown-unknown` and copies `cantrip_wasm.wasm` to the web client's
`Platforms/WebAssembly/wwwroot/engine/`, which is not tracked. `test.cmd` runs it before a run that includes the
browser's tests. The module is about 52 MB, 3.9 MB gzipped and 2.3 MB with Brotli.

## In the web client: `?local`

`?local` plays the tactical bot with nothing but the page (`MainPage.Local.cs`). The page:

1. Loads the engine (`BrowserEngine.LoadAsync`).
2. Deals a game through `HostAndPlay.StartTable`, the table the tools play on, with a `NativeRulesHostFactory` whose
   transport is `BrowserEngineTransport`: `NativeRulesHost` exactly as a server runs it, its requests answered by the
   module instead of a process.
3. Joins its own seat and the bot's over `LoopbackTransport`s. The bot is `AutoSeat` with the bot `?bot=` names (the
   tactical bot by default), stepped on a timer.

`&deck=` and `&opponent=` pick from `decks/demo`'s two-color decks, which are built into the page. `&seed=` fixes the
shuffle, and `&engine=` loads the module from another address. Card art is the page's own origin's (`/art/...`), as the
deck editor's is.

`Cantrip.Client.Web` therefore references `Cantrip.Client.Embedded` (see [client-boundary.md](client-boundary.md)). The
page's `LinkerConfig.xml` keeps `Cantrip.Server`, `Cantrip.Protocol` and `Cantrip.Core` whole, because `NativeRulesHost`
reads and writes the host's JSON by reflection.

## Answering as the native host does

A game played in a page can be checked by playing it again on a server, so the two engines must never differ.
`crates/cantrip-wasm/check/determinism.mjs` deals every scenario to both: `cantrip-native-host` over its stdin, and the
module in Node as a page loads it. No seat is driven, so the script plays the whole game. Both get the same requests
(deal, start, each seat's view, close), and every reply must be byte for byte the same:

```
node crates/cantrip-wasm/check/determinism.mjs target/wasm32-unknown-unknown/wasm/cantrip_wasm.wasm target/cards/cantrip-native-host.exe scenarios
```

On 2026-11-23 all 28,725 scenarios answered identically. The same corpus, built for `wasm32-wasip1` and run in Node at
V8's default stack (the same limit a browser tab has), passed in full with no stack overflow. Card code recurses deep
enough that the native host runs on a 256 MB thread, but no scenario comes near a browser's limit.
