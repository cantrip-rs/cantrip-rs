# In-person play

In-person play is two players at one physical table, each with a phone, and one or two tablets laid between them. A
player's phone holds what only that player may see - the hand, a library search, a private choice - and every prompt
that player answers. The tablets show the public board: what anyone at the table may look at. One machine runs the
server; every other device browses to it over the local network or a phone's hotspot.

The event is a two-player grid draft, then the games. The remote web client is unchanged by any of it: in-person play
has its own entry point, its own server process and its own wire, and nothing in the remote client's path reads them.

## Device layouts

- **Three devices.** One shared tablet lies between the players: seat 2's half at the top, turned to face seat 2, the
  stack in the middle, seat 1's half at the bottom.
- **Four devices.** Each player has a tablet of their own: that seat's half of the board, the stack, and both players'
  life totals.

On the shared tablet the stack band is mirrored: the stack is drawn twice, one strip facing each player, so both read
every spell and ability upright on the strip nearest them, in the same order, with the top of the stack at their right
and a target arrow drawn from the strip nearer what it points at. A card can't read upright for two people facing each
other, and one strip turned sideways reads well for neither; two strips cost the halves a little height, and a card held
up to read is larger anyway. Either strip's cards are tapped as the battlefield's are.

A seat's graveyard pile and, while it has cards in exile, its **Exile** button on a tablet's rail open a list of that
pile over the seat's battlefield, turned with the half: every card as the public sees it (a card exiled face down is a
back), the latest first, at the largest size that fits. A card in it is tapped as a permanent is - a creature card in a
graveyard as a spell's target - and held up to read with a long press; **Close**, or the pile again, shuts it, and it
shuts itself once the pile is empty.

A target or a mana source on the battlefield is picked by tapping it on whichever tablet shows it. A player casting
Lightning Bolt from their phone taps a Mountain on their own tablet to pay, then the opponent's Tarmogoyf on the
opponent's tablet to target it. All feedback - which taps were taken, what is still asked - is on the acting player's
phone; a tablet never shows anything of a private prompt.

## The grid draft

The draft is played on the tablets, not the phones. A tablet shows the grid face up, a button beside each row and over
each column, and beside it exactly what the remote client shows both players of a grid draft: which grid of how many,
whose pick it is and whether it is the first or the second from this grid, how many cards each player has drafted, the
last picks in words (each naming the cards it took) and the pick clock, when there is one. Nothing more: no pool, and
not the cube's pick order the draft tells each seat for its bots (`PublicGridMessage`). A pick just made is shown for a
moment - its line lit on the grid it was taken from - before the grid moves on. A card is held up to read with a long
press; a tap on a card picks nothing.

- **Two taps.** A row or a column is picked in two taps, so a stray touch takes nothing: the first lights the line and
  chooses it (the line the pick clock would take, as the remote client's choice is), the second on the same button - or
  **Take** - takes it.
- **Only the picker.** Only the face in front of the player whose pick it is has its buttons live. The relay hands a line
  picked on a tablet (`TableLineMessage`) to the phone of the seat whose pick it is (`SeatLineMessage`), and the phone
  sends the table the very pick its own grid would (`GameClient.ChooseLine`, `GameClient.PickLine`), if the draft it has
  been shown is at that pick and it is its own; so the table, judging every pick as it judges a remote seat's, stays the
  only authority, and nothing new is trusted. A player's own tablet picks only for that player; a line for a pick already
  past, or one with no card left, goes nowhere; and with the picker's phone away it goes nowhere either, the tablet's
  buttons coming back after a few seconds.
- **The shared tablet** shows the draft twice, as its stack band shows the stack: one face upright for seat 1 at the
  tablet's bottom edge, one turned to face seat 2. Each player reads the grid upright on the face in front of them and
  picks there. A grid cannot read upright for two people facing each other, and one turned sideways reads well for
  neither.
- **Four devices.** Each player's tablet shows the grid, upright for its player, its buttons live on that player's pick:
  each picks on the board in front of them, and either can read the grid while the other picks.

The phone shows **Pick on the table** meanwhile: whose pick it is, how many cards each has, and its own pool so far as
readable rows grouped by mana value, as the deck builder groups them; a tap on a row shows the card whole
(`InPerson/PhonePickScreen.cs`). Deck building then follows on the phone (docs/web-client.md, "In-person play").

The relay learns where the draft stands from the event itself, on the table's thread: the table is given an
`ObservedTableEvent` wrapping the `DraftPod`, which forwards every member of `ITableEvent` (a reflection test holds it to
that) and, whenever the table shows a seat the event, shows the relay what the event shows nobody's seat
(`ViewFor(0)`): the grid draft with no pool, both players named. The relay sends the tablets that whenever it changes
other than in the time left, and once more when the draft is over (`Grid` zero), and catches a late tablet up with it.

## Architecture

- **The public viewer.** The engine's host projects the game for a viewer who is no player, in a handle space of its
  own: it sees exactly what every player at the table may see, and no seat's hidden information. A game dealt with
  `NativeRulesHostFactory.PublicView` carries this projection on each `RulesStep` as `Public` (`PublicProjection`). A
  `GameSession` ignores it.
- **The relay.** `Cantrip.Server.InPerson` runs the table on the existing `GameServer`, wrapping each game's rules host
  in one that hands every step's public projection and outstanding requests to the relay. The relay sends tablets the
  public feed alone and routes their taps.
- **Tap routing.** A tablet's tap names an object by its public handle, or a player. The relay gives it to the seat with
  the earliest outstanding request, translated into that seat's handle space through the projection's handle map, and
  the seat's phone says whether its prompt took it. A tap the phone did not use goes to the next request; a tap with no
  request outstanding does nothing.
- **Credentials.** A phone joins its seat with the seat ticket from its QR code (`?inperson=phone&server=...#ticket=T`)
  and plays through the ordinary `/game` socket, in a phone layout of the ordinary client. A tablet's QR code holds a
  device token the relay minted (`?inperson=tablet&side=N&art=...#device=D`); tablets talk only to the relay's
  `/inperson/socket`. A welcomed device keeps a resume id and a 256-bit proof for its tab, compared in constant time;
  scanning a QR code again takes the device's place over.
- **Setup.** A setup page, served on loopback only since its QR codes hold credentials, takes the players' names, the
  cube, the packs and the layout, starts the draft and shows a QR code for each device and which are connected. Match
  and pick clocks are off by default, since the players are at the same table; a checkbox on the page turns them on.
- **Transport.** The devices speak plain WebSocket over the local network. That is accepted for a table whose devices
  share one room's network; the credentials still keep a stray device on it from taking a seat.
- **Strict tablets.** A tablet shows nothing of a private prompt: no highlighted candidates, no tap accepted or
  refused, no hint of what is being chosen.

## Full screen

Every in-person screen - the phone's game, draft and deck building, a tablet's board and its screen before the game -
has a **Fullscreen** button, a small square of four corner marks over the page (`InPerson/FullScreen.cs`): at the top of a
phone, right of the middle, and at a tablet's top right corner. Its script is `FullScreen.js`, loaded with the page as
`Keys.js` is, and it touches only a page opened with `?inperson=`: the remote client gains no manifest, no meta tag and
no change to its viewport (`FullScreenTests` checks that).

- **Android, iPad and desktop browsers** have the Fullscreen API. A tap asks for the whole screen
  (`requestFullscreen`, which a browser grants only from a tap), then locks a tablet to landscape and a phone upright
  with `screen.orientation.lock` where the browser allows one (Android's, once full screen; iPadOS and desktops have
  none, and the page lays itself out for whichever way it is held). The corners turn inward while the page is full screen
  (**Exit full screen**), and the button follows the browser however full screen was left (`fullscreenchange`). It is
  not shown where the browser has no Fullscreen API, nor in an app opened from the Home Screen, which has the screen
  already.
- **An iPhone** has no Fullscreen API outside video; its full screen is the page added to the Home Screen and opened as a
  web app. An in-person page gives iOS what it reads for that, added to the page's head by the script: a manifest of its
  own, `inperson.webmanifest` (never Uno's PWA manifest, which installs a service worker; see `Cantrip.Client.Web.csproj`),
  `"display": "fullscreen"` with icons from `cantrip.png`, `apple-mobile-web-app-capable`, a black-translucent status bar,
  an `apple-touch-icon`, and `viewport-fit=cover`. The button explains Add to Home Screen (Share, then Add to Home
  Screen) instead of doing nothing.
- **The notch and the home indicator.** With `viewport-fit=cover` the page lies under them; the script reads
  `env(safe-area-inset-*)` from an element padded by them, and each in-person page pads itself by those insets, following
  them as the device turns.
- **Coming back from the icon.** A home-screen app has storage of its own, apart from Safari's, and the page's ticket
  was taken out of the address as it loaded, so the app would know no seat. Before explaining, the button asks the relay
  for a **re-entry code** (`ReentryRequestMessage`): 256 bits, kept by the relay only as a SHA-256 hash, good once, and
  revoked when a newer one is asked for, when the seat's ticket takes the phone's place anew, when the host revokes it
  (`InPersonRelay.RevokeReentry`) or when the table closes. The page puts it after its address's `#` (`#reenter=...`);
  the manifest names no `start_url`, so the icon opens that address whether iOS takes it from the page or from the
  manifest. The app's first launch (`navigator.standalone`) brings the code in its hello; the relay compares it against
  every phone's in constant time, takes the phone's place for the app (the Safari tab is told so, and does not open its
  line again to take the place back) and welcomes it with a fresh seat ticket (`DraftPod.IssueTicket`). The app keeps
  that ticket in its own `localStorage` and loads again with it, taking its seat at the table as a scanned code would;
  every later launch brings the kept ticket, the spent code in its address ignored. Re-entry is for phones: tablets are
  iPads or Android, which have the Fullscreen API.

  Until it is used, the code is in the Safari tab's address and history on the player's own phone; that is what the
  icon is made of. A tab reloaded with it does not spend it: only an app opened from the Home Screen does. The app's first
  launch meets the table's refusal of a seat without a ticket for the moment before it has traded its code.

What needs a real iPhone to confirm, since no desktop browser behaves as iOS does: that Add to Home Screen honours the
manifest and the meta tags the script adds after the page has loaded, and makes the icon of the address with its
`#reenter=` code; that the app opens with `navigator.standalone` and its own storage, under a black-translucent status
bar; and that the safe-area insets pad the page clear of the notch and the home indicator. `FullScreenTests` plays the
iPhone's side in Chromium with the Fullscreen API taken away, an iPhone's user agent, and `navigator.standalone` for the
app, in a context of its own for the app's separate storage.

## Running a table

`tools\in-person.cmd` builds the shipping web client if it is out of date (the publish of
`play-release-build.ps1 -BuildOnly`), builds `dotnet/src/Cantrip.Server.InPerson` (`cantrip-in-person`), starts it and
opens its setup page, `http://localhost:5200/setup`. `-Event ID` hosts an event made before, with fresh codes for every
device; `-Address IP` names the address the devices reach this computer at; `-NoBuild` and `-Fast` are as
`play-release-build.cmd`'s. The event is kept in a data directory (`-Data`, else `CANTRIP_DATA`, else `.cantrip-data`).

One machine serves three ports, all on every interface, so Windows asks once for each to be let through its firewall:

| Port | What |
| --- | --- |
| 5200 (`--port`) | the site: the published web client at `/`, the relay's socket at `/inperson/socket`, and the setup page at `/setup`, to loopback alone |
| 7790 (`--table-port`) | the table: an ordinary `GameServer` hosting the event (`/game`) |
| 7791 | card art (`Cantrip.Art.ArtServer`), which the table tells its clients by the chosen address |

The setup page takes the two players' names, the cube (any list in `docs/cube-lists`), the number of grids, the layout
(three devices or four) and the address the devices use: its choices are this computer's private IPv4 addresses, those
on a network with a gateway first and virtual adapters (Hyper-V, WSL, VPNs) last (`LanAddress`), or `--address`. Start
makes the grid draft (`DraftPod.CreateGrid`), issues each player a ticket, mints a token per tablet, and starts the art
server and the table; the page then shows each device's link as a QR code (QRCoder, drawn as SVG on the server) and
whether that device is connected. The layout and the clocks are kept with the event (`EventRecord.HostData`), so hosting
it again lays out the same devices.

- A phone's link: `http://LAN:5200/?inperson=phone&server=ws%3A%2F%2FLAN%3A7790%2Fgame#ticket=T`. The phone plays the
  draft, deck building and its games through `/game` with that ticket, as any seat of an event does, and opens
  `/inperson/socket` with the same ticket for the taps routed to it.
- A tablet's link: `http://LAN:5200/?inperson=tablet&side=N&art=...#device=D`, `art` being where card art is, since a
  tablet is sent no table message to say so. A tablet only ever opens `/inperson/socket`.

Either device finds the relay beside the page that served it (`DeviceConnection.AddressFor`: `inperson/socket` in the
page's own folder, `wss://` for an `https://` page), so the same links work on the standalone server and on the bot's
site.

**Clocks.** Off by default: no pick clock in the draft (`DraftTiming.GridPick` zero), no match clock carried by the
event, and a table clock of a week (`GameServerOptions.PlayerClock` must be more than nothing, so off is one no game
runs out), with deck building given four hours and the absences that forfeit a remote player (`NoShow`, `Away`) twelve,
since a phone asleep on the table is not a player gone. The checkbox gives the event's usual clocks instead.

## Hosted in person

The Discord bot's site hosts in-person tables too, on the cloud server behind its Cloudflare tunnel: anyone opens
`https://<public-url>/inperson`, with no Discord sign-in, and sets up a table there. It is the same code, kept in
`Cantrip.Server.InPerson` and mounted by one call in the bot's `ClientSite` (`InPersonTables.Map`); docs/discord-bot.md,
"In person", has the bot's side.

- **The page.** `/inperson` (`hosted.html`, in the site's look) takes the two names, the cube, the grids, the layout and
  the clocks; there is no address to pick, since the devices reach the public URL. Its API is
  `/inperson/api/choices`, `/inperson/api/start`, `/inperson/api/table` and `/inperson/api/cancel`; the relay's socket
  is `/inperson/socket`.
- **A table already going.** Before showing the form the page looks for its visitor's open table - the signed-in
  account's (the server keeps who made a table, and `choices` answers `mine`), then the ones this browser made (their
  ids and keys in `localStorage`). Finding one, it says so and offers **Rejoin** (the table's codes page) or **Cancel**
  in place of the form; a signed-in account is refused a second table while one is going. The codes page has a cancel
  button too. Cancelling takes a second press, and `/inperson/api/cancel` takes JSON only, the table's key or its
  signed-in owner, so another site's page cannot call a table off with the creator's cookie. Nobody signed in, in
  another browser, is nobody the site can tell from a stranger: that table is reached only from its own browser or
  link, or closes on its own (below).
- **Links.** A phone's: `https://<public>/?inperson=phone&server=wss%3A%2F%2F<public>%2Ftable%2F{port}%2Fgame#ticket=T`;
  a tablet's: `https://<public>/?inperson=tablet&side=N&art=https%3A%2F%2F<public>%2F#device=D`. Without `--public-url`
  they use the bot's `--public-host` and web port, and the table's own port.
- **Tables** take a port from the bot's draft ports by the same rule as its other draft tables (a port another table
  holds refuses, and the next is tried), and tell their seats the bot's art service, whose cache is the bot's.
- **Sign-in.** The table proxy lets a WebSocket through to a port an in-person table holds with no Discord sign-in,
  for as long as it holds it (`TableAccess.TicketOnly`); the table checks the ticket itself. Every other table behind
  sign-in wants a signed-in account with a seat, as before.
- **Who sees the codes.** Making a table is open to anyone, but its codes seat whoever scans them, so they are shown only
  to its creator. Start answers the table's id and a 256-bit creator key; the page puts both after the `#` of its own
  address (`/inperson#table=ID&key=K`), which no request carries, and asks for the codes with them in a POST body, so
  neither is in an address a log keeps. The server keeps the key only as its SHA-256 hash, compared in constant time.
  The page's address, the browser that made the table, or - for a table made signed in - the creator's account are the
  only ways back to the codes.
- **Limits.** At most `--in-person-games` tables at once (2); another is refused with "All in-person tables are busy".
  A table no device has come to within 15 minutes, or that every device has left for 30, is closed, its devices told
  and its event called off; so is one whose event has been over for 10 minutes, and, when the bot starts, any event it
  left live. Names are 1-32 printable characters, the cube is one of `docs/cube-lists` by name, the grids 1-60 and no
  more than the cube holds, the layout three or four devices, and a form's body at most 4 KB. The players are given ids
  of their own (`inperson-...`), never one a typed name would make. The relay's socket takes at most twelve
  connections a table.

The standalone `cantrip-in-person` keeps its loopback-only setup page; the bot's site has no `/setup`.

## The relay

`InPersonRelay` sits between the table's game and the devices, with no socket of its own (`InPersonSite` serves it at
`/inperson/socket`, a connection going to the relay whose place its hello names) and a `TimeProvider` for its one timeout. The table deals each game through
`ObservedRulesFactory(new NativeRulesHostFactory { PublicView = true }, relay)`; the `ObservedRulesHost` it wraps each
game in forwards every member of `IRulesHost` - a reflection test holds it to that - and hands every step to the relay
on the table's thread, where the relay only takes a snapshot and queues messages.

- **Tablets** are welcomed with their side, then sent the public feed: a `WelcomeMessage` for seat zero when a game
  starts, a `StateMessage` per step (the view, which the host has stamped with `ObjectFingerprint`, and the cards the public sees for the
  first time - `PrintedCards`, the bookkeeping a `GameSession` keeps for each seat), an `EventsMessage` when the public
  saw something happen, and the `GameOverMessage`. A tablet that joins late gets the game as it stands with every card
  so far. Before the first game, and whenever who is connected changes, it gets a `DevicePhaseMessage`: the event's
  stage and a line of where it stands, read from `EventStatusBoard` once a second, never from the `DraftPod`.
- **Taps.** A `TableTapMessage` goes to the earliest outstanding request (by request id) whose seat has a phone
  connected and names the object tapped - through that step's handle map - as a `SeatTapMessage` in the seat's own
  handles. The phone answers `TapResultMessage`; unused, or unanswered within three seconds, the tap goes to the next
  request it has not been offered to, and with none left it goes nowhere. A tap before a game, after it ends, or with
  nothing outstanding does nothing. Nothing about a tap is ever sent back to a tablet.
- **Credentials.** A phone's hello carries its seat ticket, judged by the event (`DraftPod.SeatFor`); a tablet's its
  token, kept as a SHA-256 hash and compared against every minted one in constant time. A welcome carries a fresh resume
  id and 32-byte proof; a later hello with them takes the place back (constant-time compare). A ticket or token hello
  takes a place over from the device holding it, which is told so and closed, and the old proof stops working.
  A welcomed phone may ask for a re-entry code, and a hello bringing it takes the phone's place once, welcomed with a
  fresh seat ticket ("Full screen", above). Anything before a hello, or a hello in another protocol version, is refused and the connection closed.

`InPersonConfig` (`dotnet/src/Cantrip.Client.Web/InPerson`) reads what a page is: `?inperson=phone|tablet`, `?side=N`
(zero or absent for the shared board), and the tablet's `#device=` token.

## The wire

The relay and its devices speak MessagePack, in `Cantrip.Protocol.InPerson`, with unions and a version of their own
(`InPersonProtocol.Version`, now 3). A device's hello also carries `PlayerView.ProtocolVersion`, since the public feed
wraps the ordinary server messages.

A device sends a `DeviceClientMessage`:

| Key | Message | Fields |
| --- | --- | --- |
| 0 | `DeviceHelloMessage` | `InPersonVersion`, `ProtocolVersion`, `Ticket?` (a phone's), `Device?` (a tablet's), `ResumeId?`, `Proof?`, `Reentry?` (a home-screen app's first launch) |
| 1 | `TableTapMessage` | `Sequence` (how far the tablet has been brought), `Object` (a public handle, or none), `Player` (a seat, or zero) |
| 2 | `TapResultMessage` | `TapId`, `Used` |
| 3 | `TableLineMessage` | `Decision` (the pick of the grid draft), `Line` (a row 0-2 or a column 3-5), `Take` (false chooses it) |
| 8 | `ReentryRequestMessage` | none: a welcomed phone asks for a re-entry code |

The relay sends a `DeviceServerMessage`:

| Key | Message | Fields |
| --- | --- | --- |
| 0 | `DeviceWelcomeMessage` | `Role` (`Phone` or `Tablet`), `Seat` (zero for a tablet), `Side` (zero for the shared board, or the seat whose half it is), `ResumeId`, `Proof`, `PlayerNames`, `Ticket?` (a fresh seat ticket, for a phone welcomed by its re-entry code) |
| 1 | `PublicFeedMessage` | `Message`: a `WelcomeMessage` for seat zero, a `StateMessage`, an `EventsMessage` or a `GameOverMessage`, in the public handle space |
| 2 | `SeatTapMessage` | `TapId`, `RequestId` (the request it was routed to), `Object` (in the seat's handle space, or none), `Player` |
| 3 | `DevicePhaseMessage` | `Phase`, `Line` (where the draft or deck building stands), `Connected` |
| 4 | `DeviceErrorMessage` | `Code`, `Detail` |
| 5 | `PublicGridMessage` | `Decision`, `Grid` (zero once the draft is over), `Grids`, `Pick`, `Active`, `Places` (the grid row by row, a place taken an unknown card), `Drafted` (each seat's count), `Recent`, `RecentCards`, `LastLine`, `PickLeftMs` |
| 6 | `SeatLineMessage` | `Decision`, `Line`, `Take`: a line picked on a tablet, for the picker's phone |
| 8 | `ReentryCodeMessage` | `Code` (good once, for the home-screen app the phone is being added as) |

The public feed never holds a prompt, a request id or a seat's handle, and the grid nothing a seat alone is shown. A phone acts on a `SeatTapMessage` only while
its `RequestId` is the phone's prompt.

On the host's side, a step with a public view carries:

```csharp
public sealed record PublicProjection(
    PlayerView View,                                         // Viewer 0
    IReadOnlyList<(long Sequence, GameEventView View)> Events,
    IReadOnlyDictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>> Handles); // seat -> public -> seat handle
```

A handle pair is there only while the object is live and the seat's handle names the same object.

## Testing

`InPersonTests` (browser, wall clock: `test.cmd /filter "FullyQualifiedName~InPersonTests"`) draws the tablets fed by the
test, and plays whole tables end to end through `InPersonTable`: a scenario's table dealt through
`ObservedRulesFactory(new NativeRulesHostFactory { PublicView = true }, relay)`, the relay served at its socket beside the
built client, Bot1's phone (`ScenarioPlayer` with `atTable`) seated with a reserved ticket, and real tablets joined with
minted tokens - one shared board, or one for each player. Every permanent, player or graveyard card the phone means is
tapped with a finger on whichever tablet shows it, found by the public handle the step's map pairs with the seat's; the
relay routes the tap to the phone and the table judges each answer against the script. The tablets are checked never to
name a card in the phone's hand.

`InPersonDraftTests` (browser, wall clock: `test.cmd /filter "FullyQualifiedName~InPersonDraftTests"`) drafts a short grid
draft end to end on a real `InPersonHost`: both phones opened from their links, showing **Pick on the table**, and the
tablets - one shared board, or one for each player - where every pick is made with a finger, chosen with one tap and
taken with a second or with **Take**; the face of the player not picking is checked dead, each pick checked in the
event's own record, and deck building follows on the phones. `InPersonGridTests` (Server.Tests) drives the relay and a
real `DraftPod` through `ObservedTableEvent` with no socket: what the tablets are sent of the grid, and that a line goes
to the picker's phone alone.
