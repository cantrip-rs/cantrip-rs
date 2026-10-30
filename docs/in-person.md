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
