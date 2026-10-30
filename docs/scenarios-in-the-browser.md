# Scenarios at a table and in the browser

A `.cantrip` scenario (`scenarios/README.md`) is played two ways, and means the same thing both ways:

- **Headless**, by the scenario runner alone (`crates/cantrip-script`): `cargo test`, or `corpus` for reports. Every
  seat is answered by the script.
- **At a table**, through the native host and the real server, with some seats **driven**: a browser page in the seat,
  driven by Playwright, clicks its way to each answer. The script answers every other seat.

What a line means - the answer it gives, the default when it does not apply, the `?` checks, the refusals and
`[Expect]` - is written once, in Rust, in the runner. At a table the runner does not answer a driven seat's question; it
says what the script means, and the seat's answer must be exactly that, or the scenario fails at the line. So a browser
run proves every line of the scenario can be reached by clicking, and fails pointing at the step of the UI that cannot.

## The pieces

| Piece | Where | What it does |
| --- | --- | --- |
| `Scripted` | `cantrip-script/src/run.rs` | the runner with driven seats: `advance` plays on to a driven seat's question (`Advance::Ask` with the `Intent`), a client check (`Advance::Check`) or the end (`Advance::Done`); `answered` takes the seat's answer and fails at the line if it is not the one meant; `verdict` judges `[Expect]` |
| `Session::scenario` | `cantrip-host/src/session.rs` | a table playing a scenario: dealt as the runner deals it, the runner's game swapped with the table's for each move; each step carries `Scenario` (below) |
| `payload_of` | `cantrip-host/src/answer.rs` | the engine's answer as the wire's, in the driven seat's handles: the inverse of `translate` |
| `NewGame` with `Scenario` | `docs/host-protocol.md` | `{"Name": ..., "Text": ..., "Driven": [1]}` |

`every_scenario_ends_the_same_with_its_seats_driven` (cantrip-script) and
`every_scenario_passes_at_a_table_with_bot1_driven` (cantrip-host) play the whole corpus both ways and require the same
result: what the host says to answer is what the script means, over the wire and in the seat's own handles.

## A step's `Scenario`

```
"Scenario": {
  "Expected": {"Seat": 1, "RequestId": 7, "Answer": {AnswerPayload}, "Undo": false, "Line": "14: Bot1.Hand.Island.Play"},
  "Checks": [{"Line": 20, "Key": "Client.Bot1.Stack", "Value": "Black Lotus"}],
  "Verdict": null
}
```

- `Expected` is the driven seat's question now and the answer the script means (`Undo` for `Bot1.Undo`, with no
  `Answer`); `Line` is null for a default (a pass, no attack) or a payment the line in progress implies.
- `Checks` are the `? Client....` lines reached since the last step, for the driving pages to judge now.
- `Verdict` comes once the scenario is over: `{"Passed": true, "Client": [...]}`, the `[Expect]` lines about clients
  still to be judged by the pages, or `{"Passed": false, "Report": "..."}` with the runner's failure report.

None of this is for the client: the server keeps it to itself and hands it to whatever drives the seats.

## The browser driver

`ScenarioPlayer` (`dotnet/tests/Cantrip.Client.Web.Tests`) starts a `ScenarioTable` (a real server on a loopback port,
`Cantrip.Server.Host`), opens Bot1's page and, for each `Expected`, clicks there with real mouse events at the places the
page publishes under `?test` (`MainPage.Test.cs`: every card with its handle and row, each player's box and pool, every
control): the card by its handle, a menu's row by the colour it adds, the pool's mana, a target, a player's box, the
button. A pass is the pass key. An answer it cannot click its way to yet fails naming it and what the page offered; it
grows prompt by prompt.

Nothing on the page may answer for the player, or a strict run would fail where the page passed by itself: the page
opens with auto-mana off (a menu asks the colour), a stop at every step of both players' turns, and Hold Priority on. A
question the page did answer by itself - a pass it would make for anyone - is noted, and judged by the table like a
click. A test of the stops themselves (`ScenarioPlayer.PlayAsync` with `stopsMine`/`stopsTheirs`) opens the page with the
stops a player would set instead, and priority not held, so the page passes by itself wherever those stops let it, and a
pass where the script means an action fails at that line; with `opponentThinks`, Bot2 is a protocol client at the table
too, answering what the script means after a pause, as an opponent at a real table does.

A card in exile or in a graveyard is drawn only while that zone is open, so the driver opens it as a person does - the
Exile button over the piles, the graveyard pile - to click a card there or to read what its face says
(`Client.Bot1.Exile.Lightning Bolt.Annotation`), and shuts it again after a read. A face-down card the page may look at (CR
406.3) has no name on its face, and is read by what it really is, as the script names it. A menu of a card's abilities is told
apart by the ability's index, which the page publishes for each row under `?test`.

Objects nobody could tell apart count as one (`Run::same_but_for_interchangeable`): a click on the top of a pile of
untapped Forests answers what the script means by "a Forest", and the script's own answer is the one given, so later
checks of `Forest#1` mean what they say.

Tapping the sources planned for X (`MainPage.TapPlanned`), the page spends plain mana already in the pool before it taps
the next one, while the pool alone does not cover the rest - though the script names the next tap first (Fireball's
planned Mountains). The runner accepts that `AssignMana` only when the same `PayCosts` question offers both it and the
named activation, gives it, and leaves the activation line for the next question (`Run::answered`). `ScenarioPlayer`
needs no step for it: the page sends it unasked.

`ScenarioInBrowserTests` plays every scenario marked `# Browser`, or `CANTRIP_SCENARIO=name` alone (with
`CANTRIP_WEB_HEADED=1`, to watch: `tools\scenario-in-browser.cmd`). `ScenarioTableTests` (Cantrip.Server.Tests) plays
scenarios with Bot1 a protocol client sending the expected answers themselves: the table's half, so a browser run that
fails is the page's.

## Not yet

- Only Bot1 is a page; the other seats are the script's. Checks of what another seat's page shows, hidden information
  above all (that Bot2's page never shows Bot1's hand), want Bot2 driven too.
- Things a player does that are not answers to a question - a concession, Pass Turn, a stop toggled - have no lines yet.
- The driver clicks its way to priority actions (a card menu's row by the colour it adds or by `Ability:` text), payments, may,
  card choices, library searches, modes, a choice of player (the button named for them), targets (and "No target"), attacks
  and blocks, and types a card name into the name
  box; other prompts fail as not driven yet.
- A `? Client.` line is judged against the page at the next question the script reaches, so a check of what a prompt says
  follows the answer that causes it and any `Bot1.Pass` that lets it resolve (`Bot1.Pass` then `? Client.Bot1.Prompt.Title = ...`).
- A dock button's state is `? Client.Bot1.Button.<label>.Enabled = true|false` (the label holds the text; `absent` with no such button), read from
  the dock list the page publishes under `?test`. It is judged at the question the check reaches, before the answer clicks: the blockers' button
  reading `No blocks` and disabled while a creature must block is `Button.No blocks.Enabled = false`.

## Client expectations

A key starting `Client.` is about what a client shows, not the game's state: what the stack row holds, whether a
button is offered, that a card in an opponent's hand stays hidden, that a revealed card is shown, that a concession
ends the game on screen. A headless run has no client and skips them; at a table they go to the driving pages, in
`? ` checks as they come up and in `[Expect]` at the end.
