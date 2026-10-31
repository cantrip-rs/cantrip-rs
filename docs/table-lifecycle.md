# Local table membership and reconnect

Wire protocol 51 separates table membership from WebSocket lifetime. The listener is loopback unless its host sets
`ListenAddress` (the Discord bot does, on request); this is one in-process table, not an account service,
matchmaking platform or process-crash recovery system.

## Membership

A new socket first sends `JoinMessage` (current protocol version, printable 1–32-character name, optional empty-seat
preference) or `ResumeMessage` (version, membership ID, reconnect proof). A seat preference is not authentication;
the old `/game?seat=N` connection path is refused. No snapshot, table catalog or private prompt is sent before a
successful handshake. The server assigns a random opaque membership ID and independent 256-bit reconnect proof,
sent only to that member in `MembershipMessage`. Public table messages contain neither credential.

Credentials are bearer secrets. Keep them only in the owning client, never in URLs, logs or presentation text.
Loopback is the default. Any non-local deployment must deliberately add an authenticated/TLS transport and
deployment hardening; these credentials do not make plaintext remote WebSockets safe. They survive socket loss
only, not server process loss. Reconnect does not replay or relabel a historical game.

In-person play ([in-person.md](in-person.md)) is the one host that listens on the local network over plain WebSocket by
design: its table, its art and its relay (`/inperson/socket`) are on every interface, for phones and tablets in the same room.
That is accepted for one room's network and nothing wider. The seats are still taken only by the event's tickets,
which travel in the QR codes of a setup page served to loopback alone, and a tablet only by a token the relay minted.

The latest successfully authenticated resume replaces the previous socket. Replacement is atomic on the session
pump: queued commands from the superseded socket lose authority, and that socket is aborted. Wrong proofs,
another player's seat preference and an expired membership cannot claim the seat. Proofs are compared in constant
time. Protocol mismatches are refused before membership or game projection.

## Seat tickets

Protocol 102 adds a ticket to `JoinMessage`. A host calls `GameServer.Reserve(seat, name, deck)`, which fixes that
seat's name and deck and returns a 256-bit ticket, and hands it to that player alone. A join carrying the ticket
lands in that seat under the reserved name and deck, ready; `ConfigurePlayerMessage` from a reserved seat is refused.
With `RequireTickets` a join without a valid ticket is refused, so the address alone seats nobody; without it, an
open join still never lands in a reserved seat. Tickets are compared in constant time.

A ticket claims its empty seat during setup, or its own seat while that seat's connection is lost, during setup or
play. That is how a player whose client died, and so lost its reconnect proof, gets back in. It never displaces a
connected player. `TicketTransportTests` covers each case over real sockets. The Discord bot
([discord-bot.md](discord-bot.md)) is the host that uses this.

## Event tables

A table given `GameServerOptions.Event` (an `ITableEvent`, such as a `DraftPod`) belongs to an event rather than to a
lobby ([events.md](events.md)). It seats only the event's players, by the event's tickets, and in no other way: a join
without one is refused. Its phase is the event's - Setup while players take their seats, Drafting, DeckBuilding - until
it deals, which it does once the event is ready, with the decks the event says. Unlike a lobby's ticket, an event's
takes its seat over from a connection that still holds it, telling the old connection so and dropping it. A member of
an event is never released: grace expiry does not depart them, their membership proof does not expire, and `Leave` is
refused, so a table in play is never abandoned for a player being away.

## Lifetime

Transport loss reserves the seat for five minutes by default (`ReconnectGrace`, configurable up to one hour).
Outstanding rules requests wait; other seats can still answer their own requests. In a lobby, disconnect also
clears that member's readiness. Reconnect sends a fresh public table state and, if playing, that seat's authorized
game view, current event cursor and outstanding prompt. A pending private choice retains the same request ID and
handles. Historical animation events are not resent. An answer accepted before disconnect cannot execute twice:
its old request is stale. An unanswered request remains answerable after reconnect.

Lobby grace expiry releases the seat and revokes its proof. Explicit `LeaveMessage` does so immediately. If a
member departs or its grace expires during play, the table becomes `Abandoned` and no further rules answers are
accepted. This is not a concession, fabricated victory or engine `GameOverMessage`; remaining clients receive
the abandonment state and reason. `GameServer.Finished` completes on either a rules result or abandonment.
Completed games remain resynchronizable until the hosting process disposes the server. The command-line host
normally exits when `Finished` completes. Disconnected socket resources are reclaimed independently of membership.

## Setup

Normal hosting creates a lobby for 2–4 players. `--decks` supplies an allowlisted catalog; repeated `--deck` values
restrict that catalog rather than assigning positional seat decks. Catalogs contain at most 128 uniquely named
decks, each 1–2000 cards. Deck cards resolve against loaded content at startup. Clients select catalog IDs, never
server paths or scripts. Deck IDs are public table setup; deck contents and private zones are not transmitted.

Players use `ConfigurePlayerMessage` for their own name/deck and `ReadyMessage` to opt in. The lowest occupied seat
is host and uses `ConfigureTableMessage` to select up to sixteen loaded set codes. Empty selection means the base
catalog; selection applies the content library's set overrides, not a new deck-legality format. Unknown or
conflicting sets are refused. All members must be unready to change player configuration or sets; unready first
to edit. All seats must be occupied, connected and ready before dealing. Once started, setup is immutable.

`AutoStart` is explicit fixture/tool compatibility: it allows deterministic table factories and positional decks
in tests/tools. It is false by default and the normal CLI does not enable it. `GameClient` exposes lobby commands,
table state and private membership storage; a resumed client uses a fresh transport and `Resume`.

## Deterministic evidence

`MembershipTransportTests` uses real Kestrel and WebSockets. An explicitly constructed Preordain position stops
during its private library choice, disconnects, rejects a hijack, resumes matching prompt bytes/handles/context,
then repeats an answer and verifies one draw only. Other cases cover live socket replacement, protocol refusal,
catalog/name/set/readiness validation, lobby release and proof revocation, and active abandonment. Expiry tests
advance an injected clock rather than sleeping for a grace period or searching RNG seeds.

A failed setup factory leaves every member unready in the lobby and can be retried; it publishes no partial game
and does not kill the table pump. Once abandoned, the server discards queued game output, and the client refuses
late prompts so a pending question cannot reappear after the abandonment notice.
