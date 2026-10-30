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

A target or a mana source on the battlefield is picked by tapping it on whichever tablet shows it. A player casting
Lightning Bolt from their phone taps a Mountain on their own tablet to pay, then the opponent's Tarmogoyf on the
opponent's tablet to target it. All feedback - which taps were taken, what is still asked - is on the acting player's
phone; a tablet never shows anything of a private prompt.

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
  device token the relay minted (`?inperson=tablet&side=N#device=D`); tablets talk only to the relay's `/inperson`
  socket. A welcomed device keeps a resume id and a 256-bit proof for its tab, compared in constant time; scanning a QR
  code again takes the device's place over.
- **Setup.** A setup page, served on loopback only since its QR codes hold credentials, takes the players' names, the
  cube, the packs and the layout, starts the draft and shows a QR code for each device and which are connected. Match
  and pick clocks are off by default, since the players are at the same table; a checkbox on the page turns them on.
- **Transport.** The devices speak plain WebSocket over the local network. That is accepted for a table whose devices
  share one room's network; the credentials still keep a stray device on it from taking a seat.
- **Strict tablets.** A tablet shows nothing of a private prompt: no highlighted candidates, no tap accepted or
  refused, no hint of what is being chosen.

## Running a table

`tools\in-person.cmd` builds the shipping web client if it is out of date (the publish of
`play-release-build.ps1 -BuildOnly`), builds `dotnet/src/Cantrip.Server.InPerson` (`cantrip-in-person`), starts it and
opens its setup page, `http://localhost:5200/setup`. `-Event ID` hosts an event made before, with fresh codes for every
device; `-Address IP` names the address the devices reach this computer at; `-NoBuild` and `-Fast` are as
`play-release-build.cmd`'s. The event is kept in a data directory (`-Data`, else `CANTRIP_DATA`, else `.cantrip-data`).

One machine serves three ports, all on every interface, so Windows asks once for each to be let through its firewall:

| Port | What |
| --- | --- |
| 5200 (`--port`) | the site: the published web client at `/`, the relay's socket at `/inperson`, and the setup page at `/setup`, to loopback alone |
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
  `/inperson` with the same ticket for the taps routed to it.
- A tablet's link: `http://LAN:5200/?inperson=tablet&side=N#device=D`. A tablet only ever opens `/inperson`.

**Clocks.** Off by default: no pick clock in the draft (`DraftTiming.GridPick` zero), no match clock carried by the
event, and a table clock of a week (`GameServerOptions.PlayerClock` must be more than nothing, so off is one no game
runs out), with deck building given four hours and the absences that forfeit a remote player (`NoShow`, `Away`) twelve,
since a phone asleep on the table is not a player gone. The checkbox gives the event's usual clocks instead.

## The relay

`InPersonRelay` sits between the table's game and the devices, with no socket of its own (`InPersonSite` serves it at
`/inperson`) and a `TimeProvider` for its one timeout. The table deals each game through
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
  Anything before a hello, or a hello in another protocol version, is refused and the connection closed.

`InPersonConfig` (`dotnet/src/Cantrip.Client.Web/InPerson`) reads what a page is: `?inperson=phone|tablet`, `?side=N`
(zero or absent for the shared board), and the tablet's `#device=` token.

## The wire

The relay and its devices speak MessagePack, in `Cantrip.Protocol.InPerson`, with unions and a version of their own
(`InPersonProtocol.Version`, now 1). A device's hello also carries `PlayerView.ProtocolVersion`, since the public feed
wraps the ordinary server messages.

A device sends a `DeviceClientMessage`:

| Key | Message | Fields |
| --- | --- | --- |
| 0 | `DeviceHelloMessage` | `InPersonVersion`, `ProtocolVersion`, `Ticket?` (a phone's), `Device?` (a tablet's), `ResumeId?`, `Proof?` |
| 1 | `TableTapMessage` | `Sequence` (how far the tablet has been brought), `Object` (a public handle, or none), `Player` (a seat, or zero) |
| 2 | `TapResultMessage` | `TapId`, `Used` |

The relay sends a `DeviceServerMessage`:

| Key | Message | Fields |
| --- | --- | --- |
| 0 | `DeviceWelcomeMessage` | `Role` (`Phone` or `Tablet`), `Seat` (zero for a tablet), `Side` (zero for the shared board, or the seat whose half it is), `ResumeId`, `Proof`, `PlayerNames` |
| 1 | `PublicFeedMessage` | `Message`: a `WelcomeMessage` for seat zero, a `StateMessage`, an `EventsMessage` or a `GameOverMessage`, in the public handle space |
| 2 | `SeatTapMessage` | `TapId`, `RequestId` (the request it was routed to), `Object` (in the seat's handle space, or none), `Player` |
| 3 | `DevicePhaseMessage` | `Phase`, `Line` (where the draft or deck building stands), `Connected` |
| 4 | `DeviceErrorMessage` | `Code`, `Detail` |

The public feed never holds a prompt, a request id or a seat's handle. A phone acts on a `SeatTapMessage` only while
its `RequestId` is the phone's prompt.

On the host's side, a step with a public view carries:

```csharp
public sealed record PublicProjection(
    PlayerView View,                                         // Viewer 0
    IReadOnlyList<(long Sequence, GameEventView View)> Events,
    IReadOnlyDictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>> Handles); // seat -> public -> seat handle
```

A handle pair is there only while the object is live and the seat's handle names the same object.
