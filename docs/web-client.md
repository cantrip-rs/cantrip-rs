# Web client

The client, in `dotnet/src/Cantrip.Client.Web`: Uno Platform on WebAssembly, drawing with Skia, in the browser. It plays
through `Cantrip.Client` and `Cantrip.Protocol` against `Cantrip.Server.Host` over WebSocket, and takes what a card
offers, when to pass, how the log is set out and the canned prompt positions from `Cantrip.Client.Presentation`. None
of the three can reach the rules, a dataset or the server; `PresentationBoundaryTests` and `DependencyClosureTests`
hold them to it.

## Running it

`tools\play-game.cmd` is the quickest way to play: a server, the web client, a browser tab per person
(`?seat=N&name=...`) and a bot or a language model for each other seat (`dotnet\tools\Cantrip.Bot`). By hand:

```
dotnet run --project dotnet\src\Cantrip.Server.Host -- --port 7777 --players 2 --auto-start --deck demo/3-green-red --deck demo/4-red-black
dotnet run --project dotnet\src\Cantrip.Client.Web -f net10.0-browserwasm
```

Then open the app URL it prints. The page takes `?server=ws://host:port/game`; without it, the page's own host and
port 7777. Where card art is, the table says (`TableMessage.Art`, from `GameServerOptions.Art`), so a page finds it
however it got to the table - a link, a move to another table, a ticket opened on another computer. Only when the table
says nothing - a prompt fixture, a host not told - does the page go by its own `?art=...`, and failing that the art
service on the port after the table's. It is in `Cantrip.sln` and builds with it;
the first WebAssembly build downloads Emscripten through the Uno SDK.

A reload resumes the tab's seat: the membership proof is kept in `sessionStorage` and sent back with `Resume`. A
connection lost mid-game is resumed with five tries, two seconds apart, said in a line over the board. Either way the
server sends the whole game log again (see the log below), so nothing that happened while the page was away is missing
from it.

A seat ticket from a host such as the Discord bot comes after the hash, `#ticket=...`, where no request and no log
ever holds it, or in the query as `?ticket=...`. It is a bearer credential, so `Ticket.js` takes it out of the address
bar as the page loads, and a reload resumes from the membership it bought.

## The shipping build

`tools\play-release-build.cmd` plays a game in the Release build - compiled ahead of time to WebAssembly, which starts the
table about ten times sooner than the interpreted dev build. It publishes the build when it is out of date, serves it
on `http://localhost:5100` with its compressed files through `dotnet\tools\Cantrip.WebHost`, starts a server with the
playtest deck (`demo/card-test`) for both seats and a bot in seat 2, and opens seat 1 in the browser. It needs the
`wasm-tools` workload. The publish is incremental: the ahead-of-time compiler recompiles only the assemblies that
changed and keeps the rest in `obj\Release`, but the app is linked again every time, so a change to the client costs
about four minutes and an unchanged tree seconds. `-Fast` publishes without ahead-of-time compiling, to its own
folder, for a quick look at a UI change in a Release build; `-NoBuild` plays on the last publish. It compresses with
gzip only: brotli is a quarter smaller but took two minutes of every publish.

Measured on an 887-card library search: the table is drawn two seconds after the page opens, and no stall is longer
than about 80 ms. The download is about 114 MB as published, most of it the ahead-of-time runtime (67 MB, 11 MB
with brotli). `WebClientFiles` (in `Cantrip.Server.Host`) sends the compressed copies to any browser that takes them;
it is what the Discord bot serves its players with, and `Cantrip.WebHost` too.

## Card art

`Cantrip.Server.Host` serves art beside the table (`--art-port`, default the table's port plus one; `0` serves none)
at `/art/{size}/{slug}.webp`, `size` one of 128, 256, 512, 1024 on the longest side. `dotnet/src/Cantrip.Art` finds the
source in `content/*/images` by slug, resizes it once, and keeps it on disk (`--art-cache`, default
`%LOCALAPPDATA%\Cantrip\art-cache`) laid out as `{size}/{slug}.webp`. Answers carry `Cache-Control` (a week), an
`ETag`, and `Access-Control-Allow-Origin: *`. `--art-log` prints one line per request.

Who illustrated each picture is `content/<dataset>/artists.json`, slug to artist (`tools/maintenance/art-credits.py`
fills in the original printing's artist for art that has none). The art service merges the datasets' into the cache
folder as `artists.json`, beside the sizes, and serves it at `/art/artists.json`; the client fetches it once and prints
the credit, after the Mana font's artist's brush, at the foot of the card. The credit goes with the art, not the card,
so a copy credits the art it shows; a status word (MONSTROUS and the like) takes its place while there is one.

A creature that arrived this turn wears the Mana font's summoning sickness mark over its art, and its tooltip says
"Entered this turn." The mark is not on the face, which is the same before and after: each battlefield row has a layer
of its own laid over it (`ArrivalsElement`), between the row and its piles' counts, that draws only the marks - rising
and falling slowly with Animate cards on, still without it - each cut away where another card stands over its own. So
the marks move every frame and the row, and every face on it, is drawn only when it changes.

The client asks for the smallest size covering the pixels it draws, holds decoded pictures for the session, fetches
each card once however many copies are on screen, and leaves repeat loads to the browser's HTTP cache.

To move art to a CDN or a static bucket, copy the cache folder (or put the CDN in front of the art service as its
origin) and tell the tables, which tell their clients: `--art https://cdn.example/art/{size}/{slug}.webp` for the
Discord bot, `GameServerOptions.Art` for any other host.

## Looking at a position, and testing

`?prompt=NAME` in the page shows one canned prompt dock instead of a game (`PromptFixtures`), and the address's
fragment changes it without a reload.

A scenario that stops at `[Handover BotN.Verb]` used to be servable to the web client to look at, played by the C#
engine; that went with the engine. The scenarios are the engine's corpus (`scenarios/`, run by `cantrip-script`);
serving one to the browser from the Rust engine is not built yet.

The table is drawn on canvases, so there is no DOM to find a card or a button in. `&test` in the page's query makes it
publish its state as JSON on `globalThis.cantripTest`, ten times a second (`MainPage.Test.cs`):

- `ready`, `busy` and `doing` - whether the table is up, and whether it is still settling: `art` on its way, `faces`
  being made, `flights` of cards in the air, `slides` of cards along a row or turning as they tap, a `redraw` pending.
- `flown` - how many cards have flown since the page opened, for a test to see that a move was animated.
- `effects` - the latest abilities shown resolving, newest last: the `feeling` of each (`Good`, `Bad` or `Neutral`),
  whether its source was `lit`, and each of its `arrows` by what it points `to` (a card's name, or `player N`) and its
  own `feeling`. `doing` says `effects` while one is being shown.
- `log` - `lines`, how many lines the game log has, and `shown`, how many of them are made as controls: only the
  lines in and around view are, however long the game.
- `over`, `outcome`, `prompt` (the question's message type), `fixture` (the prompt dock shown, under `?prompt`) and
  `answered` (the last answer that dock sent, which nothing receives: its kind and any modes chosen).
- `cards` - every card drawn, by `name` (empty for one this seat may not see), whether it is `visible` in its row, the
  middle `x`, `y` and size `w`, `h` of its box on the page, and for a card wearing a share of a division - combat damage, or a divided spell's damage or counters - the `lower`
  and `raise` steps beside it.
- `controls` - every `button`, `toggle`, `dropdown` (by its caption), `textbox` (by its placeholder) and `item` (a row
  of an open menu or list), by its `label`, with its box.
- `title`, `detail` and `dock` - what the dock says: its question, the line under it, and its buttons in order (each
  with whether it can be pressed).

So Playwright can picture the page and click by name with real mouse events, going through the same hit testing a
person's clicks do. `&auto` makes the page play its own seat, the one deterministic way or at random from
`&bot-seed=N`, for watching a whole game; `&bot=tactical` plays it as the tactical bot does.

`&bots=2` makes the page play another seat as well as its own - `&bots=2,3,4` several, `&bot-tickets=a,b` seats
claimed by ticket - each over a connection of its own and answered in the browser by the tactical bot
(`MainPage.Bots.cs`, `docs/tactical-bot.md`). This one is a player feature: it is how an opponent is played on the
machine of whoever wanted one. `tools\play-vs-bot.cmd` opens such a page.

The web client's tests are Playwright, in `dotnet\tests\Cantrip.Client.Web.Tests`. They run on the wall clock, so plain
`tools\test.cmd` leaves them out: `test.cmd /all` runs them with everything else, or `/filter` just them. They serve the client's Debug build, launch the installed Chrome or Edge (Playwright's own
Chromium otherwise), and host the server or scenario in the test process. `TablePage` is the driver: settled, card by
name, damage steps, holding a card up, controls, drop-downs, pictures. Between them they play one whole game and another against a bot the page plays itself, draw
every prompt dock, open every scenario that hands over a position, take a seat by a Discord-style ticket, and work a
few positions by hand. Pictures go under `TestResults\web`, one folder per test class. `CANTRIP_WEB_HEADED=1` shows
the browser, to watch them.

## Drafts and deck building

A table formed from a queue is a page of its own before the draft (`LobbyPanel`): each seat, taken or "Waiting for a
player...", **Leave Table**, and once every seat is taken **Ready!** with the seconds left to press it and who is
ready. A seat given up - left, or not ready in time - says so and is forgotten, so a reload does not try to take it
back; a seat moved to another table loads that table with its new ticket (`TableMove`, `Ticket.js`). The test state's
`phase` is `Forming` and `Released` for these.

An event's table shows a screen of its own before the cards are dealt (`MainPage.Event.cs`), over the table and under
a card held up to read, so every card on it is read as one on the table is. While players take their seats the lobby
says who is here. The draft shows the piles face down with their sizes and the stack, the pile being looked at lit,
and what each player has done, and on this seat's turn the pile face up with **Take pile N** and **Pass**.

Below it, and as the whole of deck building, is the pool in columns (`MainPage.Pool.cs`): lands, then mana value 0, 1,
2 and so on to 7+, with a sideboard that pulls out on the left ("Show sideboard" / "Hide sideboard"). Cards are dragged
between columns and into or out of the sideboard - which takes a drop while tucked away, lit as the card is held over
it - and a double-click moves a card between main deck and sideboard. A card carried off is gone from its column
until it is put down, goes in among the others by its own top edge against theirs (where it is seen to be, not where
the pointer is), and flies from where it was let go. The heading counts the pool, or the deck, as
"Your pool: 41 cards (7 Land, 34 Non-land, 19 Creatures)", with a card-size slider beside it, kept in localStorage.
Beside the sideboard is the **Companion** column (CR 702.139a), shown in the deck editor and in an event's deck building where the pool has a card with companion: a card with companion dropped there is the deck's companion, a sideboard card of its own, sent with the deck and chosen anew at each game's sideboarding; the server refuses a deck whose companion's condition it doesn't fulfil. In a game, a revealed companion (CR 103.2b) waits in a small **Companion** window over the battlefield - the bottom left of the player's own half, the top left of an opponent's - and clicking the player's own offers "{3}: Put [card] in hand." at sorcery timing (CR 116.2g); taken, its window goes.

Which column a card is in is the page's alone and starts again on a reload; what is in the sideboard is sent as it
changes once decks are built, and a sideboard made during the draft becomes the deck's starting one. Each card keeps
one handle from the pile it is drafted from to its column, so a pile taken flies into place, a card taken blind flies
from the stack, and a card moved along the columns slides. While building, every basic land added is a card of its own at the foot of
the lands column; each has a minus and a plus beside the columns (published to tests as "Remove Forest" and "Add Forest"),
and **Submit deck** is enabled at the event's minimum. Until it is submitted, cards are picked and dragged together as in
the constructed deck editor (`MainPage.Picks.cs`): a box drawn from no card, or Ctrl/Shift-clicks, and a picked card
taken hold of carries the others - the same after a draft, at a league's deck table and sideboarding between games,
which are all this one screen. `?auto` drafts and builds too (`DraftPlay`), and the test
state's `phase` says which screen is up. `DraftTests` plays a draft through to the deal against a bot, pictured.

A booster draft (`format` "booster") shows the table from above instead of the piles (`DraftTableElement`): an oval
with every player in their seat - this one at the bottom, so passing left goes off to its left - the packs waiting in
front of each as a little stack with the count on it, amber once three or more pile up, and arrows round the table the
way packs are going, which turns for pack 2. The pack in front of this seat is laid out in rows below it: a click
chooses a card (lit) and **Pick _name_** takes it, or a double-click takes it at once; a pack's last card is shown
on its two-second clock and then flies into the pool. The clock on the pick counts down beside the status ("0:42 to pick"), red in its last ten seconds, and the
seat's own picks so far are listed. Deck building after a booster draft, and sideboarding between the games of a match
(titled "Sideboarding for game 2: you lead 1-0"), are the same columns with the clock under **Submit deck**: the deck as
it stands is submitted when it runs out.

A grid draft (`format` "grid") shows its grid face up to both seats (`MainPage.Event.cs`), the cards at the pack
slider's size, a place already taken left empty. A button over each column and beside each row - named "Column 2",
"Row 1" for a test - chooses that line on this seat's pick, lighting its cards, and **Take row 1 (3 cards)** takes it;
only Take takes. A line with nothing left in it cannot be chosen. Everything said about the draft is in one column
beside the grid, so the grid has the window's height: the event and which grid, what each seat has drafted, whose pick
it is and its clock, Take, what each player took ("Bot took column 2 of grid 1: ...") and the card size slider. The
taken cards fly into the pool. The other player's pick is shown being made: for a moment the grid it was taken from
stays up with that line lit ("Bot takes column 2."), from `DraftMessage.LastLine`, before the grid as it now is. The
test state names the grid's cards' row "grid". `DraftTests` drafts a grid draft of three grids against a bot through the page's buttons.

In a match's games each player's box shows their match clock large in the room beside their name and life ("24:13"),
counted down here between the server's word on it (`ClockMessage`): lit while it runs - the game is waiting on that
player. In its last five minutes it is red, and the PRIORITY badge with it: a medium red, pulsing from dark red to pure
red once a second while the clock runs.

Between rounds the event's table shows the pairings (`MainPage.Pairings.cs`, phase `Pairings`): every match of the
round with its table, both players, the games each has won and how it stands ("Game 2 is being played", "Ann won
2-1"), this seat's lit; "Round 2 begins in 0:42" once the round's last match is over; and the standings with points,
record and the three tiebreakers. A player comes back here after their match and watches the rest finish; at the end it
is the final standings. **Drop**, in the header's corner, asks again before it sends `DropMessage`; it is gone once the
seat has dropped or the event is over, and in its place is the way back to the site the pod's table names (**Back to
your page**, `TableMessage.Return`, the Discord bot's player page). Every move between the pod's table and a game's table is a `TableMove`, so each game is a fresh
page. The test state has `pairings` (round, final, starts-in, each match in words). `BoosterDraftTests` drafts seat 1
against seven bots - pick, deck building, pairings - and pictures a pod played out by bots at its final standings.

## The constructed deck editor

`?decks` (what the bot's site's deck library opens at `/decks/edit`) shows the deck editor instead of a table
(`MainPage.Decks.cs`): no game server, only the site it was served from. `&deck=ID` opens one of the player's decks as it
starts, and `&format=KEY` starts a new deck in that format. The cards are the
site's catalog (`/decks/api/catalog`, a `CardCatalogView` in MessagePack): every card the engine plays, as `CardOracle`
prints it, with its Scryfall legalities and color identity (`content/mtg/legality.json`, made by
`tools/maintenance/fetch-legality.py`). Card art is the site's own `/art` unless `?art=` says otherwise.

The header has the deck list's toggle, the deck's name, its format (Standard, Pioneer, Modern, Legacy, Vintage,
Commander, Penny Dreadful), **Save**, **New deck**, **Import deck**, whether the deck is legal and why not (`DeckRules`, in
`Cantrip.Protocol/Decks`, checked as the deck changes, and the server's check from `/decks/api/check` once it answers), and **All decks**, back to the site's deck library. On the left, collapsible, are the player's saved decks by
format, the one open lit, each with a bin; signed out, a way to log in with Discord instead. On top is the search: a box
matching name, type line and rules text, then Type, the colors W U B R G and C (any of those lit), Mana value and Sort,
over results in rows - only cards legal in the deck's format, and for Commander, once a commander is chosen, only cards
within its color identity; every one found can be scrolled to, each saying how many the deck has. Only the rows in
view and two either side are laid out (`GridWindow`, in `Cantrip.Client.Presentation`), again as the view scrolls, so the
whole card pool costs what a screenful does; a new search starts from the top. Below is the deck in the draft's
columns (`MainPage.Pool.cs`): lands, mana values 0 to 7+, the sideboard pulled out on the left, and for Commander the
command zone beside it, which takes only a card that may be a commander - alone, or paired with the one there (partner,
partner with, friends forever, choose a Background, Doctor's companion).

**Import deck** asks for a decklist pasted as Magic Online or Arena export it (`DeckImport`, in `Cantrip.Protocol/Decks`):
counts and names, sets, collector numbers and foil marks ignored; Arena's `Commander`, `Companion`, `Deck` and `Sideboard`
headings (a companion to the Companion column, a `Maybeboard` left out, `About`'s `Name` naming the deck), or Magic Online's
sideboard after the first blank line or `SB:`. Names match the catalog ignoring case, a split or double-faced card by its
faces or its front alone. The deck opens as a new one, not saved - for Commander if it names a commander, else in the
format chosen. When the list names cards the engine does not play, they are listed first, with **Import anyway**
(without them) and **Cancel**.

A result is double-clicked into its column, or dragged into a column, the sideboard or the command zone, flying there
from where it was let go; a card of the deck dragged back onto the results leaves it. A double-click moves a card between
deck and sideboard, or out of the command zone. Several cards of the deck are picked together (`MainPage.Picks.cs`): a
rectangle dragged out with the mouse from no card, over the columns or the open sideboard, picks every card it touches
where that card shows (`Marquee`, in `Cantrip.Client.Presentation`), Ctrl or Shift held adding to what is picked;
Ctrl- or Shift-click picks or lets go one card; a click on no card, or a plain click on a card, lets them all go. Picked
cards wear the gold border, and dragging any of them carries them all, put down together in their order in a column,
the sideboard or back on the results; dragging a card not picked carries it alone. The results are never picked, and a
touch on no card scrolls rather than drawing a rectangle. Saving keeps the deck however it stands, legal or not. Opening a deck from
the list, or a new one, with changes unsaved asks **Save**, **Discard** or **Cancel**; the bin asks before deleting; and
the browser asks before a page with unsaved changes is left (`DeckEditor.js`). Where in the columns each card sits is
not saved: a deck opens in its default columns. The test state has `phase` `DeckEditor` and a `deckEditor` object (the
deck by zone, its problems, the decks listed, the question asked, where the command zone is, how many results are
found and laid out, the cards picked and the rectangle being drawn); `DeckEditorTests` builds,
saves, opens, edits and deletes a deck against the real deck API with a signed-in player as a header.

## An ability resolving

When an activated or triggered ability resolves (`ResolutionEffectsElement`, `MainPage.Resolutions.cs`), its source
lights up for a moment - a border that flares in with a light running once round it and a ring going out, as a picked
card's does - and an arrow, drawn as the stack's target arrows are, grows from it to each other card or player it
touched, strikes it with a flash, and draws itself in. Red is harm (damage, life lost, -1/-1 counters, power or
toughness lowered, a permanent destroyed, exiled or sacrificed), green is help (life gained, +1/+1 counters, power or
toughness raised, a token made), and gold is anything else, or both. The source takes the colour of everything it
did together. What it did is read from the events the server says it made: `AbilityResolvedEvent` names the first
of them, and carries its targets and the creatures whose power or toughness it changed, since an effect that lasts is
no event of its own. A token made points at the token, so a targeted player who was only given tokens - Generous
Plunderer's opponent - is pointed at through them. Nothing in the client knows a card.

The table is held as it was until the arrows have struck, so a creature the ability kills is struck and then dies,
rather than leaving before the arrow reaches it. Abilities resolving together are shown one after another. A source
gone from the table - a creature whose dying it was - is not lit, and its arrows set off from where the ability was on
the stack. A reload's whole log again shows nothing, and the Animate cards setting turns it off.

## Announcing X

X, or how many times a spell is kicked, is announced before its targets (CR 601.2b), and the dock's − and + step it on
the page alone (`PromptSelection.Announced`, `Announcement` for the total at each value): nothing is sent until **Okay**
announces it, as one `AnnounceAnswer`. For an {X} paid in mana, a mana source clicked during the announcement is planned
for it rather than tapped (`MainPage.Announcing.cs`), since mana abilities wait for the payment (CR 601.2g): it wears
"For X", X rises to what the pool and the planned mana reach, and a second click takes it back. Once the targets are
chosen and the payment is asked, the page taps what was planned, one source an answer, choosing each one's mana as
auto-mana would, and asks the player only for whatever is still owed.

## Declaring attackers and blockers

Creatures are declared by picking them up and then clicking what they go at (`MainPage.Selection.cs`,
`PromptSelection.Pending`). Where a creature has only one thing to attack - one opponent, no planeswalker or battle - a
click on it attacks and a second takes it back, as ever. Where some creature has a choice, a click on one picks it up
(lit, as a chosen card is), more clicks add to them, and the things they may attack light green: the opposing player's box,
each planeswalker and battle. A click on one declares every creature picked up that may attack it, so the dock's "Attack
with N" counts them; a creature already declared comes back with a click. Blockers work the same way against the attackers.
Pairs the server would refuse (a flier against a creature without reach) are left out of what is sent and the attacker is
dimmed when none of the creatures picked up may block it.

A press on an empty part of a battlefield row and a drag draws a rectangle (`SelectionOverlayElement`), and letting go
picks up every creature that may be declared whose card it touches - sent at once where there is only one thing to
attack. A press that lands on anything else on the battlefield or in the hand puts everything down, as the dock's
**Cancel** (shown while any creature is picked up) and its hotkey do. A red arrow runs from each creature picked up to the
pointer; once they are sent, the arrows run on to the edge of what they were sent at and draw themselves in. The test
state's `selected` is how many creatures are picked up, and `arrows` is in `doing` while those are being drawn in.
`CombatSelectionTests` works both through the prompt fixtures.

## Controlling another player

While this seat controls another player (CR 723: Mindslaver, Emrakul, the Promised End), that player's questions come to
this page with `PromptMessage.ForSeat`, and `GameClient.ActingSeat` is the player each is for: the dock's title says
"Deciding for Bot 2", the pass button and auto-pass read the turn as that player's, and their cards, pool and land
drops are clicked as this seat's own would be. Their hand, which this seat may look into then (CR 723.4), is face up in
the fan on their rail, and a click opens it to play from. Both rails carry the fact ("You control them during their
turn."), and the controlled seat's dock says who makes its decisions instead of waiting on itself. The fixtures
`priority-controlling` and `controlled-waiting` show both pages (`PlayerControlTests`).

## How faces reach the screen

A face is recorded, not rasterized (`Faces.cs`). `CardFace.Draw` is played into an `SKPicture`, which keeps the
operations - the art, the frame, each run of text where the layout put it - and not their pixels. Laying a face out
(fitting the name, wrapping the rules) is what costs, and it is done once per look and size and kept (`FaceCache`);
the pixels are the GPU's. Uno records every `SKCanvasElement` into a picture of its own and plays that onto a WebGL
surface each frame, so a face drawn with `DrawPicture` is rasterized there, at the size and angle it is shown: sharp
tapped, scaled or held up, with nothing to make again for a new size. Frames are the full 1005 by 1407 bitmaps,
mipmapped once by the GPU, and a two-colour blend is a shader on the second frame rather than a layer.

Only under Uno's software renderer - a browser with no WebGL - is a recording rasterized once, at twice its size, and
kept as that image, since replaying every face's operations on the CPU each frame would cost more than the layout.

Measured in the Release build against the image-per-face drawing it replaced, on `mystical-tutor-hands-over-a-search-of-a-long-library`:
a face takes 1.7 ms to make rather than 11.4, and the faces kept take 138 KB rather than 30 MB. Hovering over a
crowded board is as smooth as before, and tapping seven permanents in a row no longer stalls a frame past 33 ms.

Card faces are laid out from `content/res/templates/retro.json` at the card's own width, with its rounding and
smallest sizes, so thumbnails clip and drop lines rather than shrink their text past reading. Abilities stand
`paragraphGap` line heights further apart than lines do. Rules text keeps the regular `rulesInset` margin unless that
would cost it a size, and then takes the `tightRulesInset` one, as a crowded printed text box does.

A planeswalker's loyalty abilities carry their cost on the mana font's loyalty shields (up, down and zero), centred on
the text box's left edge (`loyaltyCentreX`, `loyaltyWidth`) and on the ability's lines, nudged a little towards the
point so the square body is what centres; the abilities share the box out evenly with a separator between them, and
the loyalty the planeswalker has is on the start shield. The tooltip draws the same shields, light on its dark panel.

A split card is drawn as it is printed: its halves (`ObjectView.Halves`, sent everywhere but the stack) stand side by
side along its length, turned a quarter turn, and a close look turns it back to read them upright. Each half is a card
as tall as the card is wide, cut down from the template rather than shrunk (`splitHalf` in the template): bands are
taken out of the frame's middle and the seams cross-faded, so the border, the name and the mana cost are the size a
whole card's are at that size, and the halves overlap by one outer border. Each half's art is its own, under its own
name (`leave`, `chance`). On the stack the card is the half that was cast and is drawn as a whole card of its own.

A Room (CR 709.5) on the battlefield is drawn the same way, its two doors as its halves (`ObjectView.Halves`, filled for any
split card that is not on the stack), and the door that is locked (`ObjectView.UnlockedDoors`, left then right) is
veiled and marked LOCKED over its still readable face (`DrawLocked`); a Room with no door unlocked has no name and shows
both doors locked. Each locked door has its own action in the Room's menu, `ActionKindView.UnlockDoor`, worded with the
door's name and cost ("Unlock Ritual Chamber {3}{B}{B}"), apart from the Room's other abilities; the tactical bot takes
it as a card's worth of text for that cost, and the language-model client is told what it is. `?test` publishes each
card's `doors` and `actions`, which a scenario reads as `Client.Bot1.Battlefield.<Room>.Doors` and `.Actions`
(`room-doors-show-locked-and-unlocked-and-the-unlock-is-offered-with-its-cost`).

A token of its own - a Food, a Clue, a Treasure, a 2/2 Goblin Shaman - is kitbashed the same way (`token` in the
template): the frame's rows are laid down again as stretches, a stretch of the art box's twice and a stretch of the rules
box's not at all, so the rules box is half as tall and the art box takes the room. A token that copies a card reads as
the card does, frame and all (`named-tokens-hand-over-beside-token-copies` shows both).

A Saga (a card with the subtype) is drawn from the template's `saga` section, on its own frames (`saga_<color>`,
`saga_gold`, `saga_land`, and two colours blended as any card's are): the chapters down a panel on the left beside a
tall illustration, the type line at the foot. Its reminder text is the one a face prints, in italics at the top of the
chapter list, since the chapters do not read without it; each chapter follows beside its badges on the ribbon
(`content/res/icons/saga_chapter.png`, one for each number, so "II, III" carries two), with
`content/res/icons/separator.png` under the reminder and between chapters. A Saga whose colour has no saga frame is
drawn as any other card. `four-sagas-hand-over-with-their-chapters-under-way` is a board of them to look at.

A Class is drawn from the template's `class` section the same way, on the Saga frames turned over left to right
(`mirrored`), so its levels run down the right beside the art. Each level is a section headed by the cost to gain it
("{1}{U}: Level 2", in the title face), which is all that marks it - a level has no badge on the ribbon; the abilities
before the first level line are level 1. On the battlefield the levels above the one the Class has (level 1 when it has none, CR 716.2d) are drawn
spent. `two-classes-hand-over-one-of-them-at-level-two` is a board of two.

An adventurer or omen card's Adventure or Omen (CR 715.2, 720.2) comes with the card as `ObjectView.InsetSpell`, printed,
and a close look holds it up beside the card as it does a double-faced card's other face, so its text can be read before it
is cast. Its row in the card's menu is the spell it casts and that spell's own cost, "Cast Charring Bite {1}{R}...", beside
"Cast Twinmaw Stormbrood"; on the stack the card is that face alone
(`twinmaw-stormbrood-menu-offers-the-creature-and-charring-bite-with-its-cost`).

## The game log

The log is the server's, not the client's. `PlayerLog` (in `Cantrip.Server`) keeps one list of lines for the whole
table, each tagged with the seats that read it, and `LogWriter` (in `Cantrip.Projection`) words each event for each
seat from the view and events that seat is sent, as they are sent: "You draw [Island]." to one seat, "Bot 1 draws a
card." to the other. A line both seats read the same way is one line. New lines go out in a `LogMessage` as they are
written; a seat that joins, reloads or reconnects is sent the whole log again, so it is complete however long the
client was away. Each line carries the event sequence it came from, the turn, whose turn and the step, and a reference
to each object it names, by the handle that seat's view gave it, for pointing at the thing while it is still on the
table. A retraction - a mana ability taken back - removes lines by number. Clients only set the lines out under turn
and step headings (`GameJournal`); the game-over panel reads its "why" from the log's loss lines.

The log is the first of the side panel's tabs (`SidePanel`): **Game log**, **Settings**, and **Menu**
(`GameMenuPanel`), which holds **Concede** and **Offer Draw**. Conceding asks again before it sends `ConcedeMessage`, and is only offered
when it would end the game - every other player still in is of one team - since a multiplayer game going on without the
player (CR 800.4a) is not supported yet (`GameEngine.CanConcede`). The test state names each tab as a control of kind
`tab`.

The **game menu** (`MainPage.GameMenu.cs`) is the same at a click: a button of three lines (named "Game menu" to the test
state) beside the panel's hide arrow, and beside the arrow that opens the panel again while it is hidden, opening a short
menu (`CardMenu`) of **Concede Game** - which opens the Menu tab to ask once more - and **Offer Draw**
(`docs/rules-exceptions.md`, RX-4). A row that cannot be taken now is drawn faint: a draw is offered only while the player
has priority, and offering passes it (`GameClient.OfferDraw`, `OfferDrawMessage`). When another player offers one
(`DrawOfferMessage`), the dock asks **Accept Draw** or **Decline** over whatever else it is asking - neither is the Pass
key's - and the menu buttons are lit until it is answered; the entry then reads **Accept Draw** while the offer is open, so a
player who declined may accept at any priority of theirs (`GameClient.CanAcceptDraw`). The Menu tab's line under the
button says where the offer stands, and to its player who has declined it. A draw agreed is said over the board as the
table says it (`TableMessage.Detail`, "The players agreed to a draw."). `GameMenuTests` opens the menu, has the other seat
offer a draw, and accepts it from the dock.

## When the game is over

The result is said over the board (`MainPage.Outcome.cs`): who won, and why, from the log's loss lines, with **Look at
the board** to put it away. A table that names the page its players came from (`TableMessage.Return`, with its name in
`ReturnName`) is gone back to from there (`GoingBack`, in `Cantrip.Client.Presentation`): the Discord bot's tables name
its draft queues (`/draft`) after a Winston or grid draft's game, and its constructed play (`/constructed`) after a
constructed game, which is a match of one game. The result then has **Back to Draft Queues** (or **Back to Constructed
Play**) above **Look at the board**, and a line under them counting down: after 30 seconds the page goes back by itself
(`Ticket.js`'s `cantripGoBack`, an http(s) address only), leaving the table in the tab's history. **Look at the board**
stops the countdown - the player is looking - and the **Menu** tab keeps the button for when they are done. A table
that names nothing - one of the player's own (`tools\play-vs-bot.cmd`, `?bots=2`), or a booster pod's game, whose
players the pod sends on to the next game or back to its pairings and standings (`TableMoveMessage`) - has the result
alone, as before.

An event over with no game of its own to say so - a Winston or grid draft called off because a player dropped, or did not
take their seat - is a page of its own where the lobby is (`LobbyPanel.RenderOver`): the event's name, why it ended
(`TableMessage.Detail`, which the table takes from the event), and the button back to the page the table names. A seat
given up at a table still filling - left, or not ready in time - has the same button back to the draft queues.
`GoingBackTests` drops a player from a draft and goes back from the other's page.

## In-person play

At an in-person table (`docs/in-person.md`) each player's phone is this page, opened from its QR code with
`?inperson=phone` and its seat's ticket, playing through the ordinary `/game` socket; the public board lies on tablets.
Everything the phone adds is in `MainPage.InPerson.cs` and `InPerson/PhoneLink.cs`, reached from one line in the
constructor (`AttachPhone`), and does nothing on any other page: the remote client is unchanged by it.

- **Layout.** Upright: the dock across the phone's width over the hand, a bar over them with the **Board** button, and
  above them this player's rail (life, mana pool, piles) with the stack and any choice window floating beside it. The
  whole board, both halves as the remote client draws them, is behind **Board**: the tablets show the public board, and
  only the phone can show its owner what they alone may see there, such as their face-down cards. The side panel starts
  closed, without touching the remote client's setting, and opens over the table. A clock with more than a day left is not
  shown (`PlayerRail.HideClockOver` for the match clocks, the draft's, deck building's and the round's lines blanked):
  an in-person table's clocks are "off" as clocks of days and hours, not as none. The draft and deck building are the
  page's own, at a phone's size (`DraftTests.A_grid_draft_is_drafted_on_an_in_person_phone`).
- **Taps on the tablets.** The relay hands the phone a tap on a tablet routed to its seat (`SeatTapMessage`, already in
  this seat's handles), and the page does exactly what a click on that card, or that player's box (`PlayerClick`), does
  here: a land tapped at priority, while a cost is paid or while X is announced; a target; an attacker's or a blocker's
  pick. It acts only on the request the tap was routed to, once that prompt has been drawn; a tap that beats its prompt
  waits about a second for it, and taps are taken in the order they came. A menu the click opens (a land's colours) opens
  in the middle of the phone. The phone says whether the tap was used (`TapResultMessage`); one it could not use - its
  prompt past, or the thing tapped no choice now - is said on the phone with a short line over the dock and a buzz where
  the browser can vibrate (`Phone.js`), and never on the tablet.
- **Testing.** Under `?test`, the test state's `inPerson` is the phone's Board, the taps waiting, every tap settled (used,
  held for its prompt) and the line up now; a test stands in for the relay by queueing taps on
  `globalThis.cantripTableTaps` (`o12@34;` the seat's object 12 for request 34, `p2@34;` player 2), since `cantripTest`
  itself is a string. `ScenarioPlayer.PlayAsync(..., phone: true)` plays a scenario on a phone-sized touch page, tapping
  the hand and the dock with a finger and tapping every permanent and player on the "tablet" through that queue, the table
  judging each answer against the script (`PhoneTests`: Bolt at a Tarmogoyf with the Mountain tapped at priority, Fireball
  at a player paid with lands tapped during the payment, Engineered Explosives' X planned with a land tapped as it is
  announced, and the taps the phone must not use).

## Open

- The emblem pile beside a player's graveyard.
- Concede, rematch, spectating, sound, and more than one opponent drawn.
