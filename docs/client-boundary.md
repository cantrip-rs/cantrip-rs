# Client presentation and hosting boundary

The client is the [web client](web-client.md). It plays against the server over WebSocket, and nothing it is
built from can reach the rules, a dataset or the server session. This is an assembly boundary, not merely a
division into partial classes.

## Components

- `Cantrip.Client` owns transport-facing client state (`GameClient`, `WebSocketTransport`), `PromptSelection` and
  the pure `CardChoicePresentation` model. It references only Core and Protocol; it neither runs rules nor loads
  datasets, and it holds no UI.
- `Cantrip.Client.Presentation` is a plain library of what the table shows and does that is not drawing: which
  actions a card offers (`CardActions`), when a seat passes by itself (`AutoPass`), how the server's log is set out
  under turn and step headings (`GameJournal`), and the canned prompt positions and their `FixtureTransport`
  (`PromptFixtures`). It references only `Cantrip.Client`.
- `Cantrip.Client.Web` is the UI: Uno Platform on WebAssembly, drawing with Skia. Its table code draws from the two above.
  It builds its `GameClient` over a `WebSocketTransport`, or over a `FixtureTransport` for `?prompt=NAME`, or, for a game
  with no server (`?local`), over a `LoopbackTransport` to a table in the page. The
  seat's membership proof stays in the tab's `sessionStorage` and goes back only in `Resume`, never into public
  lobby metadata; a seat ticket in the address is taken out of the address bar as the page loads.
- `Cantrip.Client.Embedded` owns `HostAndPlay` and `HostedGame`, including dataset/deck loading and the
  authoritative `GameSession`. Callers receive seat transports rather than the server session. The playtest and
  language-model game tools under `dotnet\tools\` reference it. So does the web client, for `?local` alone
  ([engine-in-the-browser.md](engine-in-the-browser.md)): a game with no server is the page's own table, dealt on the
  rules engine in the page, and the page plays it through seat transports like any other. A game on a server stays the
  server's; nothing in the page reaches into it.

The presentation assembly's Cantrip dependency closure is **Presentation, Client, Core and Protocol**.
`PresentationBoundaryTests` traverses compiled assembly references, and `DependencyClosureTests` holds
`Cantrip.Client` to Client, Core and Protocol.

## Prompt facts and authority

Protocol 51 adds optional `PromptMessage.Presentation` context: action, cause, chooser, owner, source handle,
origin zones, count limits and ordering. `PromptPresentationProjector` supplies these facts on the authorized
private prompt; it does not broadcast revealed choices to the other players. Context uses stable data types,
not executable UI or prelocalized sentences. Existing context-free fixtures can use compatibility wording.

Card-choice wording uses those facts to distinguish a cost from an effect and mixed zones from a single hand or
battlefield. Distribution and payment retain their validated wire payloads. The client's enabled buttons and local
selections are convenience checks; the server remains authoritative for every answer, and every answer passes
through the same `GameClient` methods.

## Testing

`PresentationBoundaryTests` and `DependencyClosureTests` guard the assembly boundary above. The client tests
(`dotnet\tests\Cantrip.Client.Tests`) cover MessagePack round trips for every prompt fixture, count-zero/one/many and
optional choices, mixed zones, cost/effect wording, replacement ordering, invalid selections and reset behavior. The
browser tests in `dotnet\tests\Cantrip.Client.Web.Tests` draw every prompt fixture and play one whole game. Serving a
scenario's handed-over position to the browser went with the C# engine and is not rebuilt yet. Never search random
seeds.

See [table lifecycle](table-lifecycle.md) for membership, setup and reconnect policy.
