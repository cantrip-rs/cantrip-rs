using Cantrip.Core.Decisions;
using MessagePack;

namespace Cantrip.Protocol;

/// <summary>
/// Anything the server sends a client.
/// <para>
/// A MessagePack union with explicit keys, so adding a message type is backward compatible and a client
/// built from a different commit fails at the handshake rather than misreading a frame.
/// </para>
/// </summary>
[Union(0, typeof(WelcomeMessage))]
[Union(1, typeof(StateMessage))]
[Union(2, typeof(PromptMessage))]
[Union(3, typeof(GameOverMessage))]
[Union(4, typeof(ErrorMessage))]
[Union(5, typeof(EventsMessage))]
[Union(6, typeof(MembershipMessage))]
[Union(7, typeof(TableMessage))]
[Union(8, typeof(LogMessage))]
[Union(9, typeof(DraftMessage))]
[Union(10, typeof(DeckBuildingMessage))]
[Union(11, typeof(SeatReleasedMessage))]
[Union(12, typeof(TableMoveMessage))]
[Union(13, typeof(PairingsMessage))]
[Union(14, typeof(ClockMessage))]
[Union(15, typeof(DrawOfferMessage))]
[Union(16, typeof(SelectionVerdictMessage))]
[Union(17, typeof(BlocksVerdictMessage))]
[Union(18, typeof(AttacksVerdictMessage))]
public abstract class ServerMessage
{
    public abstract string Describe();
}

/// <summary>Sent once when a client is seated.</summary>
[MessagePackObject]
public sealed class WelcomeMessage : ServerMessage
{
    [Key(0)]
    public int ProtocolVersion { get; set; } = PlayerView.ProtocolVersion;

    /// <summary>Which seat this client is playing. Seats are numbered from one.</summary>
    [Key(1)]
    public int Seat { get; set; }

    [Key(2)]
    public List<string> PlayerNames { get; set; } = [];

    /// <summary>
    /// This seat's own deck as the game began: every card it owned, by name, with how many, in name order. Only its own -
    /// what another player brought is theirs to reveal. Tokens and copies are made in the game, so are not in it.
    /// </summary>
    [Key(3)]
    public List<DeckCardView> Decklist { get; set; } = [];

    public override string Describe() => $"welcome, seat {Seat}";
}

/// <summary>A card in a decklist, by name, how many of it, and the card as printed.</summary>
[MessagePackObject]
public sealed class DeckCardView
{
    [Key(0)]
    public string Name { get; set; } = string.Empty;

    [Key(1)]
    public int Count { get; set; }

    /// <summary>
    /// The card as printed, its oracle, as <see cref="StateMessage.Printed"/> sends a card: for showing the deck, and for
    /// drawing its faces before any of it is dealt. It has no handle, so it names no object in the game - which copy is
    /// where in a library stays hidden.
    /// </summary>
    [Key(2)]
    public ObjectView Card { get; set; } = new();
}

/// <summary>
/// The current position as this client is entitled to see it.
/// <para>
/// A full snapshot for now. Deltas and a visibility-filtered semantic event log — which the client needs to
/// know <em>why</em> something changed in order to animate it — are an addition on top, not a replacement:
/// a periodic authoritative snapshot is still what makes reconnection, late joining, and spectating work.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class StateMessage : ServerMessage
{
    [Key(0)]
    public PlayerView View { get; set; } = new();

    /// <summary>
    /// Every card this seat can see in <see cref="View"/> for the first time this game, as printed: its oracle, the card
    /// before anything in the game has touched it, with the other face of a double-faced card. Each is sent once per
    /// game, and all of them again when a seat is brought back up to date. The client draws a card's face from it
    /// before the card is shown, and tells a card the game has changed from it.
    /// </summary>
    [Key(1)]
    public List<ObjectView> Printed { get; set; } = [];

    public override string Describe() => $"state: {View}";
}

/// <summary>
/// What has happened since this client's last update, in terms it can animate.
/// <para>
/// Sent alongside the snapshot rather than instead of it. A structural diff of two views can say a card is no longer
/// on the battlefield and is now in a graveyard, but not whether it was destroyed, sacrificed, or bounced — and those
/// look different on screen. Working the reason out client-side would mean the client inferring rules.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class EventsMessage : ServerMessage
{
    [Key(0)]
    public List<GameEventView> Events { get; set; } = [];

    /// <summary>How far through the log this client has now been brought.</summary>
    [Key(1)]
    public long Sequence { get; set; }

    /// <summary>
    /// The log sequence of each event, in the same order as <see cref="Events"/>. Sent so a later
    /// <see cref="EventsRetractedEvent"/> can name the events it withdraws: events a viewer may not see are left out of
    /// the batch, so position alone could not.
    /// </summary>
    [Key(2)]
    public List<long> Sequences { get; set; } = [];

    public override string Describe() => $"{Events.Count} event(s) to sequence {Sequence}";
}

[Union(0, typeof(ObjectMovedEvent))]
[Union(1, typeof(LifeChangedEvent))]
[Union(2, typeof(DamageMarkedEvent))]
[Union(3, typeof(TapChangedEvent))]
[Union(4, typeof(StepBeganEvent))]
[Union(5, typeof(SpellCastEvent))]
[Union(6, typeof(SpellFizzledEvent))]
[Union(7, typeof(LibraryShuffledEvent))]
[Union(8, typeof(CardsRevealedEvent))]
[Union(9, typeof(EventsRetractedEvent))]
[Union(10, typeof(LibraryArrangedEvent))]
[Union(11, typeof(AbilityActivatedEvent))]
[Union(12, typeof(TokenCreatedEvent))]
[Union(13, typeof(CountersPutEvent))]
[Union(14, typeof(ExtraTurnAddedEvent))]
[Union(15, typeof(PlayerLostEvent))]
[Union(16, typeof(GameEndedEvent))]
[Union(17, typeof(TransformedEvent))]
[Union(18, typeof(DieRolledEvent))]
[Union(19, typeof(ExtraPhaseAddedEvent))]
[Union(20, typeof(DayNightBecameEvent))]
[Union(21, typeof(MonarchBecameEvent))]
[Union(22, typeof(RandomPlayerChosenEvent))]
[Union(23, typeof(StartingPlayerChosenEvent))]
[Union(24, typeof(MulliganTakenEvent))]
[Union(25, typeof(HandKeptEvent))]
[Union(26, typeof(AbilityResolvedEvent))]
[Union(27, typeof(CoinFlippedEvent))]
[Union(28, typeof(TurnEndedEvent))]
[Union(29, typeof(CardsLookedAtEvent))]
[Union(30, typeof(ScriedEvent))]
[Union(31, typeof(SurveilledEvent))]
public abstract class GameEventView;

/// <summary>The seat chosen at random decided who takes the first turn (CR 103.1).</summary>
[MessagePackObject]
public sealed class StartingPlayerChosenEvent : GameEventView
{
    [Key(0)]
    public int Chooser { get; set; }

    /// <summary>Who plays first: the chooser, unless they chose to draw first.</summary>
    [Key(1)]
    public int Player { get; set; }
}

/// <summary>A seat took a mulligan (CR 103.5). Public, though neither hand is.</summary>
[MessagePackObject]
public sealed class MulliganTakenEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>How many mulligans they have taken, counting this one.</summary>
    [Key(1)]
    public int Count { get; set; }
}

/// <summary>
/// A coin was flipped (CR 705). Public. A flip <see cref="Called"/> is won or lost: <see cref="Won"/> is whether
/// <see cref="Player"/> won it, the way a card that says "if you lose the flip" reads it. A flip that cares only whether it
/// comes up heads is not called, and no one wins or loses it (CR 705.2): <see cref="Heads"/> is what it came up.
/// </summary>
[MessagePackObject]
public sealed class CoinFlippedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public bool Won { get; set; }

    [Key(2)]
    public bool Called { get; set; }

    [Key(3)]
    public bool Heads { get; set; }
}

/// <summary>
/// An effect ended <see cref="Player"/>'s turn (CR 724.1): every spell and ability on the stack is exiled and the turn goes
/// straight to its cleanup step. Public.
/// </summary>
[MessagePackObject]
public sealed class TurnEndedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }
}

/// <summary>
/// <see cref="Player"/> scried <see cref="Count"/> (CR 701.22): public; which cards went where is known to the scrier
/// alone.
/// </summary>
[MessagePackObject]
public sealed class ScriedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int Count { get; set; }
}

/// <summary>
/// <see cref="Player"/> surveilled <see cref="Count"/> (CR 701.25): public; the cards put into the graveyard arrive as
/// moves.
/// </summary>
[MessagePackObject]
public sealed class SurveilledEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int Count { get; set; }
}

/// <summary>A seat kept its opening hand (CR 103.5), of this many cards.</summary>
[MessagePackObject]
public sealed class HandKeptEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int Cards { get; set; }
}

/// <summary>A public random player selection, announced as the effect resolves before priority resumes.</summary>
[MessagePackObject]
public sealed class RandomPlayerChosenEvent : GameEventView
{
    [Key(0)]
    public int Controller { get; set; }

    [Key(1)]
    public int SelectedPlayer { get; set; }

    [Key(2)]
    public ViewHandle Source { get; set; }
}

/// <summary>
/// A player became the monarch (CR 725.1). Public: who the monarch is, is a fact about the game that everyone at
/// the table can see, and the view already carries it. Sent whenever it moves, whatever moved it — an effect saying
/// so, a creature getting through to the monarch (CR 725.2), or the monarch leaving the game (CR 725.4).
/// </summary>
[MessagePackObject]
public sealed class MonarchBecameEvent : GameEventView
{
    /// <summary>The player who is now the monarch.</summary>
    [Key(0)]
    public int Player { get; set; }
}

/// <summary>
/// It became day or night (CR 731.1). Public: the designation is a fact about the game, which everyone at the table
/// can see. Sent whenever it changes, whatever changed it — the untap step's own turn-based action (CR 502.2), a
/// permanent with daybound or nightbound arriving in a game that is neither (CR 702.145d, 702.145g), or an effect
/// that says so outright.
/// </summary>
[MessagePackObject]
public sealed class DayNightBecameEvent : GameEventView
{
    /// <summary>What it now is. Never <see cref="DayNightView.Neither"/>: nothing takes a designation away.</summary>
    [Key(0)]
    public DayNightView Designation { get; set; }
}

/// <summary>
/// A double-faced permanent turned over (CR 701.27a). Public, like the permanent. Both names, since the view already
/// shows the new face by the time this is read.
/// </summary>
[MessagePackObject]
public sealed class TransformedEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public string From { get; set; } = string.Empty;

    [Key(2)]
    public string To { get; set; } = string.Empty;
}

/// <summary>
/// A player lost the game, and what did it (CR 104.3). Public: everyone at the table sees a player leave and knows why.
/// </summary>
[MessagePackObject]
public sealed class PlayerLostEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public LossReasonView Reason { get; set; }
}

/// <summary>The game ended (CR 104.2a, 104.4), and how.</summary>
[MessagePackObject]
public sealed class GameEndedEvent : GameEventView
{
    /// <summary>Never <see cref="GameOutcomeView.InProgress"/>.</summary>
    [Key(0)]
    public GameOutcomeView Outcome { get; set; }

    /// <summary>The team left standing, or zero for a draw.</summary>
    [Key(1)]
    public int WinningTeam { get; set; }

    /// <summary>For a draw, why (CR 104.4).</summary>
    [Key(2)]
    public DrawReasonView DrawReason { get; set; }
}

/// <summary>
/// An activated ability was put on the stack (CR 602.2), and what it was aimed at. The counterpart of
/// <see cref="SpellCastEvent"/>: the ability has no card of its own, so its source is named too.
/// </summary>
[MessagePackObject]
public sealed class AbilityActivatedEvent : GameEventView
{
    /// <summary>The ability on the stack. None once it has already resolved by the time this is read.</summary>
    [Key(0)]
    public ViewHandle Ability { get; set; }

    /// <summary>The permanent whose ability it is.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    [Key(2)]
    public int Controller { get; set; }

    /// <summary>How the ability reads.</summary>
    [Key(3)]
    public string Text { get; set; } = string.Empty;

    [Key(4)]
    public List<TargetMessage> Targets { get; set; } = [];

    /// <summary>A triggered ability put on the stack (CR 603.3) rather than one a player activated.</summary>
    [Key(5)]
    public bool Triggered { get; set; }

    /// <summary>
    /// A modal ability's chosen modes, as their text, in the order they happen (CR 700.2d), as
    /// <see cref="SpellCastEvent.Modes"/> is for a spell; empty for an ability that is not modal.
    /// </summary>
    [Key(6)]
    public List<string> Modes { get; set; } = [];
}

/// <summary>
/// An ability finished resolving (CR 608.2m): the counterpart of <see cref="AbilityActivatedEvent"/>. The events its
/// effects made are the ones from <see cref="First"/> up to this one, so a client can show what it did - the damage, the
/// life, the counters - as coming from its source.
/// </summary>
[MessagePackObject]
public sealed class AbilityResolvedEvent : GameEventView
{
    /// <summary>The ability, which has ceased to exist; none when this viewer was never given a handle for it.</summary>
    [Key(0)]
    public ViewHandle Ability { get; set; }

    /// <summary>The object whose ability it was; none when this viewer cannot see it any more.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    [Key(2)]
    public int Controller { get; set; }

    /// <summary>The log sequence of the first event its effects could have made.</summary>
    [Key(3)]
    public long First { get; set; }

    /// <summary>What it was aimed at, of what this viewer can see.</summary>
    [Key(4)]
    public List<TargetMessage> Targets { get; set; } = [];

    /// <summary>The creatures whose power or toughness it changed, and by how much, of those this viewer can see.</summary>
    [Key(5)]
    public List<PowerToughnessChangeView> PowerToughness { get; set; } = [];
}

/// <summary>How far an effect raised or lowered one creature's power and toughness.</summary>
[MessagePackObject]
public sealed class PowerToughnessChangeView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public int Power { get; set; }

    [Key(2)]
    public int Toughness { get; set; }
}

/// <summary>A token was created and put onto the battlefield (CR 111.1).</summary>
[MessagePackObject]
public sealed class TokenCreatedEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public int Controller { get; set; }

    [Key(2)]
    public string Name { get; set; } = string.Empty;
}

/// <summary>Counters were put on a permanent (CR 122.6).</summary>
[MessagePackObject]
public sealed class CountersPutEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    /// <summary>Which kind, by name (CR 122.1): <c>+1/+1</c>, <c>lore</c>, <c>mining</c>.</summary>
    [Key(1)]
    public string Name { get; set; } = string.Empty;

    /// <summary>How many were put on it.</summary>
    [Key(2)]
    public int Count { get; set; }
}

/// <summary>A player was given extra turns after this one (CR 500.7).</summary>
[MessagePackObject]
public sealed class ExtraTurnAddedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>How many extra turns.</summary>
    [Key(1)]
    public int Count { get; set; }
}

/// <summary>A phase was added directly after this one, in the turn currently running (CR 500.8), as by an additional combat phase.</summary>
[MessagePackObject]
public sealed class ExtraPhaseAddedEvent : GameEventView
{
    [Key(0)]
    public PhaseView Phase { get; set; }
}

/// <summary>
/// A player looked at the top of their library and put the cards back (CR 701.22), or shuffled instead. Counts only:
/// which cards went where is known to nobody but the player who looked.
/// </summary>
[MessagePackObject]
public sealed class LibraryArrangedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int OnTop { get; set; }

    [Key(2)]
    public int OnBottom { get; set; }

    [Key(3)]
    public bool Shuffled { get; set; }
}

/// <summary>
/// Earlier events are withdrawn because what they reported was undone, as a mana ability taken back is.
/// <para>
/// A client removes whatever those events produced — a line in the log — rather than showing the undo as something
/// that happened. Sequences this client was never sent are simply not found.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class EventsRetractedEvent : GameEventView
{
    [Key(0)]
    public List<long> Sequences { get; set; } = [];
}

/// <summary>
/// A player looked at hidden cards: the top card of each library (Medomai's Prophecy), a card at random of a hand
/// (Urza's Bauble).
/// <para>
/// Told to every seat, but only the player who looked is shown the cards: to anyone else they are cards it may not see,
/// and the log says how many.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class CardsLookedAtEvent : GameEventView
{
    /// <summary>Who looked.</summary>
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public List<ViewHandle> Cards { get; set; } = [];
}

/// <summary>
/// Cards were revealed (CR 701.20), such as a hand shown by Duress.
/// <para>
/// Only cards this viewer can see while the reveal lasts are listed, so it never names anything the viewer was not
/// shown. The handles last as long as the reveal: once the choice it was made for is answered they are dropped.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class CardsRevealedEvent : GameEventView
{
    /// <summary>Whose cards they are.</summary>
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public List<ViewHandle> Cards { get; set; } = [];
}

/// <summary>
/// A spell was cast, and what it was aimed at (CR 601.2c).
/// <para>
/// Sent alongside the move that put it on the stack rather than instead of it: the move is what a renderer
/// animates, and this is what a log reads out. Targets are public from the moment they are announced, so they are
/// carried here — by the time anyone reads the log the spell has resolved and the stack object is gone, so there
/// is nothing left to recover them from.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class SpellCastEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Spell { get; set; }

    [Key(1)]
    public int Controller { get; set; }

    [Key(2)]
    public List<TargetMessage> Targets { get; set; } = [];

    /// <summary>The value chosen for X, for a spell with {X} in its cost; null otherwise.</summary>
    [Key(3)]
    public int? X { get; set; }

    /// <summary>A modal spell's chosen modes, as their text, in the order they happen; empty for any other spell.</summary>
    [Key(4)]
    public List<string> Modes { get; set; } = [];

    /// <summary>How many times the spell was kicked (CR 702.33); zero when it was not.</summary>
    [Key(5)]
    public int Kicked { get; set; }

    /// <summary>Whether that payment was a buyback (CR 702.27) rather than a kicker, which the log words differently.</summary>
    [Key(6)]
    public bool Buyback { get; set; }

    /// <summary>The seat its gift was promised to (CR 702.174k), which is announced with the spell; null when none was.</summary>
    [Key(7)]
    public int? GiftPromisedTo { get; set; }
}

/// <summary>
/// A spell did not resolve because every target it had chosen had become illegal (CR 608.2b).
/// <para>
/// Reported rather than left to be inferred from "it went to a graveyard and nothing happened". A player who has
/// just lost a spell to a removed target wants to be told why, and the client has no way to work it out.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class SpellFizzledEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Spell { get; set; }

    [Key(1)]
    public int Controller { get; set; }
}

/// <summary>A library was shuffled (CR 701.22).</summary>
[MessagePackObject]
public sealed class LibraryShuffledEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }
}

/// <summary>A die was rolled (CR 706). Public: everyone at the table sees what a die comes up as.</summary>
[MessagePackObject]
public sealed class DieRolledEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>How many faces the die had (CR 706.1a).</summary>
    [Key(1)]
    public int Sides { get; set; }

    /// <summary>The face that came up, from 1 to <see cref="Sides"/>.</summary>
    [Key(2)]
    public int Result { get; set; }
}

/// <summary>
/// An object changed zones.
/// <para>
/// <see cref="Object"/> is <see cref="ViewHandle.None"/> when the viewer is not entitled to know which card moved —
/// an opponent drawing, for instance. The zones and the reason are still sent, because that a card was drawn is
/// public; which card it was is not.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ObjectMovedEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public ZoneKindView From { get; set; }

    [Key(2)]
    public int FromOwner { get; set; }

    [Key(3)]
    public ZoneKindView To { get; set; }

    [Key(4)]
    public int ToOwner { get; set; }

    [Key(5)]
    public ZoneChangeReasonView Reason { get; set; }
}

[MessagePackObject]
public sealed class LifeChangedEvent : GameEventView
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int Delta { get; set; }

    [Key(2)]
    public int NewTotal { get; set; }
}

[MessagePackObject]
public sealed class DamageMarkedEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public int Amount { get; set; }
}

[MessagePackObject]
public sealed class TapChangedEvent : GameEventView
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public bool Tapped { get; set; }
}

[MessagePackObject]
public sealed class StepBeganEvent : GameEventView
{
    [Key(0)]
    public int Turn { get; set; }

    [Key(1)]
    public TurnStepView Step { get; set; }

    [Key(2)]
    public int ActivePlayer { get; set; }
}

/// <summary>Why an object moved. Mirrors the engine's reasons; the drift test covers it.</summary>
public enum ZoneChangeReasonView
{
    Draw = 0,
    Play = 1,
    Cast = 2,
    Resolve = 3,
    Discard = 4,
    Destroyed = 5,
    Sacrificed = 6,
    Exiled = 7,
    Returned = 8,
    Milled = 9,
    StateBasedAction = 10,

    /// <summary>Countered (CR 701.5). Not destroyed, and none of its effects happen.</summary>
    Countered = 11,

    /// <summary>Found by a library search (CR 701.23).</summary>
    Searched = 12,

    /// <summary>Put on top of a library from somewhere else, as Brainstorm puts cards back.</summary>
    PutOnLibrary = 13,

    /// <summary>A token put onto the battlefield as it is created (CR 111.1).</summary>
    Created = 14,

    /// <summary>Put into a graveyard from the top of a library by surveil (CR 701.25a).</summary>
    Surveilled = 15,

    /// <summary>Shuffled into a library from a hand or a graveyard, as Timetwister shuffles both in (CR 701.24).</summary>
    ShuffledIn = 16,

    /// <summary>
    /// Manifested (CR 701.40a): onto the battlefield face down, or for manifest dread's other card, into the graveyard.
    /// </summary>
    Manifested = 17,

    /// <summary>Put into a hand from a library without drawing or searching, as Dark Confidant does.</summary>
    PutIntoHand = 18,
}

/// <summary>The server needs this client to decide something.</summary>
[MessagePackObject]
public sealed class PromptMessage : ServerMessage
{
    /// <summary>Echoed back in the answer, so a reply that has been overtaken can be rejected.</summary>
    [Key(0)]
    public int RequestId { get; set; }

    [Key(1)]
    public PromptPayload Prompt { get; set; } = new PriorityPromptMessage();

    /// <summary>
    /// Asked while this client is still announcing a spell or ability, so it may answer with a
    /// <see cref="CancelAnnouncementAnswer"/> instead and give the whole thing up, leaving the game as it was before.
    /// </summary>
    [Key(2)]
    public bool MayCancel { get; set; }

    /// <summary>Structured wording context, scoped to this prompt's authorized recipient; never executable UI.</summary>
    [Key(3)]
    public PromptPresentationContext? Presentation { get; set; }

    /// <summary>
    /// The seat whose decision this is when it is not this client's own: a player this seat controls (CR 723.5), whose
    /// choices it makes with that player's resources (CR 723.5a). The prompt is in this client's handles and the options are
    /// that player's - "you" in them is that player. Zero for this seat's own decision.
    /// </summary>
    [Key(4)]
    public int ForSeat { get; set; }

    public override string Describe() => ForSeat == 0
        ? $"prompt {RequestId}: {Prompt.GetType().Name}"
        : $"prompt {RequestId}: {Prompt.GetType().Name} for seat {ForSeat}";
}

[MessagePackObject]
public sealed class GameOverMessage : ServerMessage
{
    [Key(0)]
    public GameOutcomeView Outcome { get; set; }

    [Key(1)]
    public int WinningTeam { get; set; }

    public override string Describe() => $"game over: {Outcome}";
}

/// <summary>
/// Something the client sent could not be accepted. Never fatal on its own: a rejected stale reply is a
/// normal consequence of reconnecting, not a protocol violation.
/// </summary>
[MessagePackObject]
public sealed class ErrorMessage : ServerMessage
{
    [Key(0)]
    public ErrorCode Code { get; set; }

    [Key(1)]
    public string Detail { get; set; } = string.Empty;

    /// <summary>The request the error is about, when it is about one (a refused <see cref="CheckSelectionMessage"/>); 0 otherwise.</summary>
    [Key(2)]
    public int RequestId { get; set; }

    public override string Describe() => $"error {Code}: {Detail}";
}

public enum ErrorCode
{
    /// <summary>The reply referred to a request that is no longer outstanding. Expected after a reconnect.</summary>
    StaleReply = 0,

    /// <summary>The answer was not something the prompt allows.</summary>
    IllegalAnswer = 1,

    /// <summary>A handle in the answer does not name anything this client can currently see.</summary>
    UnknownHandle = 2,

    ProtocolMismatch = 3,

    NotYourTurnToAnswer = 4,
    MembershipRequired = 5,
    MembershipRefused = 6,
    InvalidSetup = 7,
    TableClosed = 8,
}

/// <summary>Anything a client sends the server.</summary>
[Union(0, typeof(JoinMessage))]
[Union(1, typeof(AnswerMessage))]
[Union(2, typeof(ResumeMessage))]
[Union(3, typeof(ConfigurePlayerMessage))]
[Union(4, typeof(ConfigureTableMessage))]
[Union(5, typeof(ReadyMessage))]
[Union(6, typeof(LeaveMessage))]
[Union(7, typeof(DraftPickMessage))]
[Union(8, typeof(BuildDeckMessage))]
[Union(9, typeof(ConcedeMessage))]
[Union(10, typeof(DropMessage))]
[Union(11, typeof(OfferDrawMessage))]
[Union(12, typeof(AnswerDrawMessage))]
[Union(13, typeof(CheckSelectionMessage))]
[Union(14, typeof(CheckBlocksMessage))]
[Union(15, typeof(CheckAttacksMessage))]
public abstract class ClientMessage
{
    public abstract string Describe();
}

[MessagePackObject]
public sealed class JoinMessage : ClientMessage
{
    [Key(0)]
    public int ProtocolVersion { get; set; } = PlayerView.ProtocolVersion;

    [Key(1)]
    public string PlayerName { get; set; } = string.Empty;

    /// <summary>A preference for an unoccupied lobby seat, never proof of membership.</summary>
    [Key(2)]
    public int PreferredSeat { get; set; }

    /// <summary>
    /// A seat ticket the host issued for this player, or empty. Unlike <see cref="PreferredSeat"/> it is a bearer
    /// credential: it claims the one seat it was issued for, under the name and deck fixed when it was issued. Never
    /// logged or described.
    /// </summary>
    [Key(3)]
    public string Ticket { get; set; } = string.Empty;

    public override string Describe() => $"join as '{PlayerName}'";
}

[MessagePackObject]
public sealed class AnswerMessage : ClientMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    [Key(1)]
    public AnswerPayload Answer { get; set; } = new PassAnswer();

    public override string Describe() => $"answer {RequestId}: {Answer.GetType().Name}";
}

// --- Prompts ---

[Union(0, typeof(PriorityPromptMessage))]
[Union(1, typeof(DiscardPromptMessage))]
[Union(2, typeof(DeclareAttackersPromptMessage))]
[Union(3, typeof(DeclareBlockersPromptMessage))]
[Union(4, typeof(AssignCombatDamagePromptMessage))]
[Union(5, typeof(SearchLibraryPromptMessage))]
[Union(6, typeof(PayCostsPromptMessage))]
[Union(7, typeof(ChooseCardsPromptMessage))]
[Union(8, typeof(UnlessPaysPromptMessage))]
[Union(9, typeof(DistributeCardsPromptMessage))]
[Union(10, typeof(ChooseTargetPromptMessage))]
[Union(11, typeof(MayPromptMessage))]
[Union(12, typeof(ChooseModesPromptMessage))]
[Union(13, typeof(ChooseReplacementPromptMessage))]
[Union(14, typeof(OrderTriggersPromptMessage))]
[Union(15, typeof(NameCardPromptMessage))]
[Union(16, typeof(ChoosePlayerPromptMessage))]
[Union(17, typeof(ChooseColorsPromptMessage))]
[Union(18, typeof(ChooseDividedTargetsPromptMessage))]
[Union(19, typeof(ChooseStartingPlayerPromptMessage))]
[Union(20, typeof(MulliganPromptMessage))]
[Union(21, typeof(RemoveCountersPromptMessage))]
public abstract class PromptPayload
{
    /// <summary>
    /// The simplest legal answer to this question, composed only out of what the prompt itself carries.
    /// <para>
    /// The mirror of the engine prompt's own default, and abstract for the same reason: a new kind of question cannot
    /// reach the wire without saying how a client that does not understand it should answer. Every automatic
    /// answerer on this side — the client's bot, the transport tests' bots, and the dock the client shows for a prompt
    /// it has no screen for — falls back to this, so a client built before a prompt existed keeps the game moving
    /// instead of sitting on a question it cannot read.
    /// </para>
    /// <para>
    /// Built from the prompt's own lists, so it can never name a card, target, or option that was not offered — the
    /// same guarantee that keeps a bot from killing a game with an answer the server then refuses.
    /// </para>
    /// </summary>
    public abstract AnswerPayload Default();
}

/// <summary>
/// This client chooses the modes of a modal spell it is casting (CR 601.2b, 700.2), while the card is still in its hand.
/// Answered with a <see cref="ModesChosenAnswer"/>. Each chosen mode that targets is asked a
/// <see cref="ChooseTargetPromptMessage"/> next, whose text is the mode's.
/// </summary>
[MessagePackObject]
public sealed class ChooseModesPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The card being cast.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>Every mode's text, in printed order. Answers name modes by index into this list.</summary>
    [Key(2)]
    public List<string> Modes { get; set; } = [];

    /// <summary>The indexes that may be chosen: a mode with no legal target cannot be.</summary>
    [Key(3)]
    public List<int> Eligible { get; set; } = [];

    /// <summary>How many modes to choose, or the fewest of them where <see cref="AtMost"/> opens a range.</summary>
    [Key(4)]
    public int Count { get; set; }

    /// <summary>Whether the same mode may be chosen more than once.</summary>
    [Key(5)]
    public bool MayRepeat { get; set; }

    /// <summary>The choices are additional costs of which one is paid, as "discard a card or pay 3 life", not modes.</summary>
    [Key(6)]
    public bool AdditionalCost { get; set; }

    /// <summary>
    /// What is being chosen, when it is neither modes nor an additional cost and there may be no card to name: "Venture
    /// into Undercity: choose the next room". Empty otherwise.
    /// </summary>
    [Key(7)]
    public string Heading { get; set; } = string.Empty;

    /// <summary>
    /// The most modes that may be chosen, for "choose one or more" (CR 700.2, escalate). Zero where the count is
    /// exact; already narrowed to what the caster can pay the escalate cost for.
    /// </summary>
    [Key(8)]
    public int AtMost { get; set; }

    /// <summary>
    /// The eligible modes whose own cost (spree's) the caster could pay for together with the spell's. Empty where modes
    /// have no cost of their own. A mode outside it may still be chosen, but the cast will be given up while paying.
    /// </summary>
    [Key(9)]
    public List<int> Affordable { get; set; } = [];

    /// <summary>
    /// For an additional cost that is a keyword action, which: <c>Behold</c> or <c>Blight</c>. Empty otherwise.
    /// </summary>
    [Key(10)]
    public string CostKind { get; set; } = string.Empty;

    /// <summary>
    /// What the question adds to the choice, in words: that each mode past the first has a cost (escalate, spree), or that the
    /// modes are the faces a card may be cast as. Empty when there is nothing to add.
    /// </summary>
    [Key(11)]
    public string Detail { get; set; } = string.Empty;

    /// <summary>Whether choosing more modes than the fewest costs something extra (escalate, spree): a range of free modes is "one or both".</summary>
    [Key(12)]
    public bool ExtraModesCost { get; set; }

    /// <summary>
    /// The rules text of each mode, index for index, where a mode is a thing to weigh and not only to read: the faces a card may be
    /// cast as (CR 709.3) are each a card's printed text. Empty for modes the question's own text says it all of.
    /// </summary>
    [Key(13)]
    public List<string> Texts { get; set; } = [];

    /// <summary>
    /// The eligible modes in the order worth trying them: those <see cref="Affordable"/> names first, then the rest, each
    /// group in printed order.
    /// </summary>
    [IgnoreMember]
    public IEnumerable<int> Preferred => Eligible.OrderBy(mode => Affordable.Count > 0 && !Affordable.Contains(mode));

    /// <summary>The most modes an answer may name: the range's top where there is one, <see cref="Count"/> otherwise.</summary>
    [IgnoreMember]
    public int Most => AtMost > Count ? AtMost : Count;

    /// <summary>
    /// The first eligible modes, affordable ones first, repeating the last one it may when repeats are allowed and too
    /// few are eligible. The fewest a range will take, since the modes past the first are what escalate and spree charge
    /// for.
    /// </summary>
    public override AnswerPayload Default() => new ModesChosenAnswer
    {
        Modes = MayRepeat
            ? [.. Enumerable.Range(0, Count).Select(i => Eligible[Math.Min(i, Eligible.Count - 1)])]
            : [.. Preferred.Take(Count)],
    };
}

/// <summary>
/// Somewhere the cards of a <see cref="DistributeCardsPromptMessage"/> may go, and how many of them it takes.
/// <para>
/// A destination says which cards it will accept rather than why, so a client can tell whether a placement is legal
/// without knowing any rules: see <c>CardDistribution</c> in <c>Cantrip.Core</c>, which is the same check the engine
/// makes when the answer arrives.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class CardDestinationMessage
{
    /// <summary>Where cards put here go, which is what a client labels the pile with.</summary>
    [Key(0)]
    public CardDestinationKindView Kind { get; set; }

    /// <summary>Whose zone it is. Usually the player being asked.</summary>
    [Key(1)]
    public int Owner { get; set; }

    /// <summary>The fewest cards that may be put here, already reduced to what the cards there are can meet.</summary>
    [Key(2)]
    public int Min { get; set; }

    /// <summary>The most that may be, or <see cref="int.MaxValue"/> for "and the rest".</summary>
    [Key(3)]
    public int Max { get; set; }

    /// <summary>Which of the cards looked at may be put here. All of them, unless the effect narrowed it.</summary>
    [Key(4)]
    public List<ViewHandle> Eligible { get; set; } = [];

    /// <summary>
    /// Whether the order cards are put here in matters, as it does for either end of a library, and for the rules' question
    /// of the order of cards put into a graveyard together (CR 404.3), which is a lone graveyard pile, the first on top.
    /// </summary>
    [Key(5)]
    public bool Ordered { get; set; }

    /// <summary>Whether they are placed in a random order instead, so there is no order for anyone to choose.</summary>
    [Key(6)]
    public bool Randomized { get; set; }

    /// <summary>
    /// The kinds this takes one card of each, as Atraxa, Grand Unifier's "a card of each card type": every card put
    /// here has to count as a different one of these lists. Empty when it does not care.
    /// </summary>
    [Key(7)]
    public List<List<ViewHandle>> Groups { get; set; } = [];

    /// <summary>What each group is called, index for index: "Creature".</summary>
    [Key(8)]
    public List<string> GroupNames { get; set; } = [];
}

/// <summary>
/// This client looks at some cards and says where each one goes: scrying, which may send some to the bottom;
/// surveilling, which may put some into the graveyard; Ponder's "in any order", which may shuffle instead; Stock Up's
/// two into the hand and the rest on the bottom.
/// <para>
/// All of them are one message because they are one question, differing only in <see cref="Destinations"/>. Answered
/// with a <see cref="CardsDistributedAnswer"/> naming every card exactly once across lists that run index for index
/// with the destinations. The handles last as long as the prompt: once it is answered they are dropped, so nothing can
/// be tracked afterwards.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class DistributeCardsPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell or ability doing the looking.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>The cards looked at, top card of the library first.</summary>
    [Key(2)]
    public List<ViewHandle> Cards { get; set; } = [];

    /// <summary>Where they may go, in the order the card's rules text names them.</summary>
    [Key(3)]
    public List<CardDestinationMessage> Destinations { get; set; } = [];

    /// <summary>Whether shuffling instead is allowed.</summary>
    [Key(4)]
    public bool MayShuffle { get; set; }

    /// <summary>
    /// Requirements met in order, then everything left over into the first destination that will take it: the same
    /// placement the engine would fall back to, so a client that draws no dock for this still answers legally.
    /// </summary>
    public override AnswerPayload Default() => new CardsDistributedAnswer
    {
        Placements = CardDistribution.Greedy(Cards, Limits(), Accepted(), Groups()),
    };

    /// <summary>
    /// Why this placement is not one the server will take, or null when it is — the same question
    /// <c>DistributeCardsPrompt.Accepts</c> asks, from the same code, so a client can grey its Done button on exactly
    /// what would be refused rather than on a guess at the rule.
    /// </summary>
    public string? Problem(CardsDistributedAnswer answer)
    {
        ArgumentNullException.ThrowIfNull(answer);

        return CardDistribution.Problem(
            Cards,
            Limits(),
            Accepted(),
            [.. answer.Placements.Select(placement => (IReadOnlyList<ViewHandle>)placement)],
            Groups());
    }

    private IReadOnlyList<IReadOnlyList<ViewHandle>>[] Groups() =>
        [.. Destinations.Select(destination =>
            (IReadOnlyList<IReadOnlyList<ViewHandle>>)[.. destination.Groups.Select(group => (IReadOnlyList<ViewHandle>)group)])];

    private DestinationLimit[] Limits() =>
        [.. Destinations.Select(destination => new DestinationLimit(destination.Min, destination.Max))];

    private IReadOnlyList<ViewHandle>[] Accepted() =>
        [.. Destinations.Select(destination => (IReadOnlyList<ViewHandle>)destination.Eligible)];
}

/// <summary>
/// This client chooses cards from among <see cref="Cards"/>, every one of which it sees: its own discards, a card from an
/// opponent's hand revealed to it (Duress), permanents for a cost, cards looked at.
/// <para>
/// Which sets of the cards are an answer is the rules' to say, by the question's selection rule (docs/engine-design.md,
/// "Selection rules"): <see cref="Verdict"/> is its word on choosing nothing yet - the cards that may be picked first, and
/// whether none at all is an answer - and every selection after is checked with the host as it changes
/// (<see cref="CheckSelectionMessage"/>), which says whether it is an answer and what may be added to it.
/// <see cref="Count"/>, <see cref="CountKind"/>, <see cref="Kind"/> and <see cref="Words"/> are for wording the question.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ChooseCardsPromptMessage : PromptPayload, ISelectionPrompt
{
    /// <summary>Who is choosing.</summary>
    [Key(0)]
    public int Player { get; set; }

    /// <summary>Whose hand the cards are in.</summary>
    [Key(1)]
    public int Owner { get; set; }

    /// <summary>The spell asking.</summary>
    [Key(2)]
    public ViewHandle Source { get; set; }

    [Key(3)]
    public CardChoicePurposeView Purpose { get; set; }

    /// <summary>The cards chosen from, in the question's order: what <see cref="Verdict"/> and every check name cards of.</summary>
    [Key(4)]
    public List<ViewHandle> Cards { get; set; } = [];

    /// <summary>How many the choice takes, as <see cref="CountKind"/> counts it, for wording it.</summary>
    [Key(6)]
    public int Count { get; set; }

    /// <summary>What kind of card is being chosen ("artifact"), where the purpose alone does not say. Empty otherwise.</summary>
    [Key(8)]
    public string Kind { get; set; } = string.Empty;

    /// <summary>
    /// The total power the chosen cards must add up to, as saddle's "total power N or greater" (CR 702.171a); zero
    /// for a choice that asks nothing of it. <c>Cantrip.Core.Decisions.TotalPower</c> is the rule.
    /// </summary>
    [Key(9)]
    public int MinimumPower { get; set; }

    /// <summary>The power of each of <see cref="Cards"/>, index for index, when <see cref="MinimumPower"/> is set.</summary>
    [Key(10)]
    public List<int> Powers { get; set; } = [];

    /// <summary>
    /// For <see cref="CardChoicePurposeView.TapToReduce"/> (harmonize, CR 702.180a): each of <see cref="Powers"/> is the most
    /// tapping that creature could take off the generic mana of the total cost, which X, a kicker or the targets announced
    /// after the creature is chosen may still change (CR 601.2b-f); false when it is exactly what tapping it takes off.
    /// </summary>
    [Key(25)]
    public bool PowersUpTo { get; set; }

    /// <summary>
    /// The total mana value the chosen cards must add up to, as collect evidence's "total mana value N or greater"
    /// (CR 701.59a); zero for a choice that asks nothing of it. <c>Cantrip.Core.Decisions.TotalManaValue</c> is the rule.
    /// </summary>
    [Key(11)]
    public int MinimumManaValue { get; set; }

    /// <summary>The mana value of each of <see cref="Cards"/>, index for index, when <see cref="MinimumManaValue"/> or <see cref="MaximumTotalManaValue"/> is set.</summary>
    [Key(12)]
    public List<int> ManaValues { get; set; } = [];

    /// <summary>
    /// The most a cast chosen here may be worth: the mana value of the spell cast must be at most this, as what is left of Uldaros
    /// Theorix's "total mana value 6 or less". Zero when there is no limit.
    /// </summary>
    [Key(13)]
    public int MaximumManaValue { get; set; }

    /// <summary>For <see cref="CardChoicePurposeView.PutCounters"/>, the kind of counter the chosen permanent gets, as Oracle text names it ("loyalty", "+1/+1"). Empty otherwise.</summary>
    [Key(14)]
    public string Counter { get; set; } = string.Empty;

    /// <summary>For <see cref="CardChoicePurposeView.PutCounters"/>, how many counters the chosen permanent gets.</summary>
    [Key(15)]
    public int CounterCount { get; set; }

    /// <summary>
    /// The fewest that may be chosen when <see cref="CountKind"/> makes <see cref="Count"/> a maximum: craft's "one or more"
    /// (CR 702.167a), for wording it. Zero when any number up to <see cref="Count"/> will do.
    /// </summary>
    [Key(16)]
    public int Minimum { get; set; }

    /// <summary>
    /// "With total mana value N or less" (CR 202.3), as Michelangelo's Technique's "up to two creature cards with total mana value
    /// 6 or less": the most the chosen cards' <see cref="ManaValues"/> add up to, for the running total a client shows. Null when
    /// the question's rule keeps no such count. Display alone: the rule's verdicts judge the cards.
    /// </summary>
    [Key(17)]
    public int? MaximumTotalManaValue { get; set; }

    /// <summary>For <see cref="CardChoicePurposeView.PreventDamageFrom"/>, how much damage the shield has left to prevent (CR 615.7).</summary>
    [Key(18)]
    public int PreventLeft { get; set; }

    /// <summary>For <see cref="CardChoicePurposeView.PayLifeOrPutOnTop"/>, the life each chosen card costs to keep (Sylvan Library's 4).</summary>
    [Key(20)]
    public int Life { get; set; }

    /// <summary>
    /// For <see cref="CardChoicePurposeView.Band"/>, the eligible creatures without banding, of which a band may have at most
    /// one (CR 702.22c).
    /// </summary>
    [Key(19)]
    public List<ViewHandle> WithoutBanding { get; set; } = [];

    /// <summary>How the choice counts its cards: exactly <see cref="Count"/>, up to it, any number, or as many as there are.</summary>
    [Key(22)]
    public CountKindView CountKind { get; set; }

    /// <summary>What the question's rule asks of the cards beyond their noun, each in the words a caption puts after it ("with total mana value 6 or less", "with different names").</summary>
    [Key(23)]
    public List<string> Words { get; set; } = [];

    /// <summary>The rule's verdict on choosing nothing yet: whether that is an answer, and the cards that may be picked first.</summary>
    [Key(24)]
    public SelectionVerdictView Verdict { get; set; } = new();

    [IgnoreMember]
    public IReadOnlyList<ViewHandle> Candidates => Cards;

    /// <summary>For wording: <see cref="Count"/> is a most, fewer allowed ("up to three lands", or "any number").</summary>
    [IgnoreMember]
    public bool UpTo => CountKind is CountKindView.UpTo or CountKindView.AnyNumber;

    /// <summary>For wording: "any number of" (CR 608.2d), none, some or all, rather than "up to" so many.</summary>
    [IgnoreMember]
    public bool AnyNumber => CountKind == CountKindView.AnyNumber;

    /// <summary>
    /// The first cards that may be picked, as many as it takes (a rule of a count alone takes them; one asking more of the
    /// cards together is settled with the host, <see cref="CheckSelectionMessage"/>). A choice that must add up to a power is
    /// offered "up to" every candidate, so taking every one with positive power meets it whenever anything can (a negative
    /// power takes from the total, CR 107.1b); one with a <see cref="Minimum"/> takes that many.
    /// </summary>
    public override AnswerPayload Default() => MinimumPower > 0
        ? new CardsChosenAnswer { Cards = [.. Cards.Where((_, i) => i >= Powers.Count || Powers[i] > 0)] }
        : MaximumTotalManaValue is { } most
            ? new CardsChosenAnswer { Cards = FirstFitting(most) }
            : new CardsChosenAnswer { Cards = [.. Verdict.Addable.Take(Minimum > 0 ? Minimum : Count)] };

    /// <summary>
    /// "With total mana value N or less" (CR 202.3): the first cards that may be picked and fit together, in order, as many
    /// as it takes - the first <see cref="Count"/> of them alone could add up to more (a 3 and a 2 under 4). Each card's mana
    /// value is at its place among <see cref="Cards"/>.
    /// </summary>
    private List<ViewHandle> FirstFitting(int most)
    {
        List<ViewHandle> taken = [];
        foreach (ViewHandle card in Verdict.Addable)
        {
            int at = Cards.IndexOf(card);
            int value = at >= 0 && at < ManaValues.Count ? ManaValues[at] : 0;
            if (taken.Count < Count && value <= most)
            {
                most -= value;
                taken.Add(card);
            }
        }

        return taken;
    }
}

/// <summary>
/// A question that picks a set of cards by a selection rule (<see cref="ChooseCardsPromptMessage"/>,
/// <see cref="SearchLibraryPromptMessage"/>, and a <see cref="ChooseTargetPromptMessage"/> whose targets together are
/// judged): what a client reads of it to let a player pick, every selection after the first checked with the host
/// (<see cref="CheckSelectionMessage"/>).
/// </summary>
public interface ISelectionPrompt
{
    /// <summary>Every card that may be picked from, in the question's order.</summary>
    IReadOnlyList<ViewHandle> Candidates { get; }

    int Count { get; }

    CountKindView CountKind { get; }

    /// <summary>The rule's verdict on choosing nothing yet.</summary>
    SelectionVerdictView Verdict { get; }

    /// <summary>
    /// Whether the question is judged by a rule at all: always for a choice of cards and a library search; for a choice of
    /// targets, only where the slot judges its targets together (<see cref="ChooseTargetPromptMessage.Verdict"/>).
    /// </summary>
    bool Judged => true;
}

/// <summary>How a choice of cards counts them: what its caption says of <c>Count</c>.</summary>
public enum CountKindView
{
    /// <summary>Exactly that many.</summary>
    Exactly = 0,

    /// <summary>Up to that many, none included.</summary>
    UpTo = 1,

    /// <summary>Any number, none and all included (CR 608.2d, 701.23).</summary>
    AnyNumber = 2,

    /// <summary>That many, or fewer only when no more can be found (a search for "a card", CR 701.23b).</summary>
    AsManyAsCan = 3,
}

/// <summary>What a question's selection rule says of a set of cards picked: whether it is an answer, and which may be added.</summary>
[MessagePackObject]
public sealed class SelectionVerdictView
{
    /// <summary>The cards picked are an answer the rules take.</summary>
    [Key(0)]
    public bool Complete { get; set; }

    /// <summary>The cards not picked that may be added to them, in the question's order. Taking one back out is always allowed.</summary>
    [Key(1)]
    public List<ViewHandle> Addable { get; set; } = [];

    /// <summary>
    /// In a prompt's first verdict, for a question that takes one card at most: the <see cref="Addable"/> cards that are, picked
    /// alone, an answer. A client may send such a pick at once, without asking the host (<see cref="CheckSelectionMessage"/>);
    /// empty in any other verdict and for a question of more cards.
    /// </summary>
    [Key(2)]
    public List<ViewHandle> CompleteAlone { get; set; } = [];
}

/// <summary>
/// "Check this partial selection for the pending question": what the question's selection rule says of the cards a player
/// has picked so far, asked as the selection changes, answered with a <see cref="SelectionVerdictMessage"/>. Nothing in the
/// game moves; the answer itself is still checked when it is given. For a <see cref="ChooseCardsPromptMessage"/>, a
/// <see cref="SearchLibraryPromptMessage"/>, or a <see cref="ChooseTargetPromptMessage"/> with a
/// <see cref="ChooseTargetPromptMessage.Verdict"/> (its object targets picked so far) only.
/// </summary>
[MessagePackObject]
public sealed class CheckSelectionMessage : ClientMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    /// <summary>The cards picked so far, in the order picked.</summary>
    [Key(1)]
    public List<ViewHandle> Selected { get; set; } = [];

    public override string Describe() => $"check {RequestId}: {Selected.Count} picked";
}

/// <summary>
/// The rules' verdict on a selection a <see cref="CheckSelectionMessage"/> asked about: whether the cards picked are an
/// answer, and which may be added to them.
/// </summary>
[MessagePackObject]
public sealed class SelectionVerdictMessage : ServerMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    /// <summary>The selection the verdict is on, as it was asked.</summary>
    [Key(1)]
    public List<ViewHandle> Selected { get; set; } = [];

    [Key(2)]
    public SelectionVerdictView Verdict { get; set; } = new();

    public override string Describe() => $"verdict {RequestId}: {(Verdict.Complete ? "complete" : "incomplete")}, {Verdict.Addable.Count} addable";
}

/// <summary>
/// What the rules make of a declaration of blockers (CR 509.1a-c), as the host's one validator judges it: whether it may be
/// declared as it is, why not, and the blocks that some legal declaration extending it has. Every restriction and requirement
/// is the host's to weigh, so a client asks it (<see cref="CheckBlocksMessage"/>) rather than working the rules out.
/// </summary>
[MessagePackObject]
public sealed class BlockVerdictView
{
    /// <summary>The declaration may be made as it is.</summary>
    [Key(0)]
    public bool Complete { get; set; }

    /// <summary>
    /// The blocks not declared that some legal declaration extending it has: one blocker of a menace creature is addable
    /// though alone it is no declaration, and a block no legal declaration has alongside the ones declared is not. Taking a
    /// block back out is always allowed.
    /// </summary>
    [Key(1)]
    public List<BlockMessage> Addable { get; set; } = [];

    /// <summary>Why it may not be declared as it is; null when it may.</summary>
    [Key(2)]
    public BlockIssueView? Issue { get; set; }

    /// <summary>
    /// What declaring it costs (CR 509.1d), in mana symbols ("{2}"); empty for a declaration that costs nothing. The cost is
    /// paid once the declaration is sent, through the payment prompt attack costs use (<see cref="PayCostsPromptMessage"/>).
    /// </summary>
    [Key(3)]
    public string Cost { get; set; } = string.Empty;
}

/// <summary>
/// Why a declaration of blockers may not be made as it is, to caption as <see cref="Creature"/>, <see cref="Words"/>, then
/// <see cref="Other"/> where it is not <see cref="ViewHandle.None"/> ("Grizzly Bears" "must block" "Ox"; "Ox" "can't be
/// blocked except by two or more creatures"), with the Comprehensive Rules number of the rule it breaks ("509.1c").
/// </summary>
[MessagePackObject]
public sealed class BlockIssueView
{
    [Key(0)]
    public ViewHandle Creature { get; set; }

    [Key(1)]
    public string Words { get; set; } = string.Empty;

    [Key(2)]
    public ViewHandle Other { get; set; }

    [Key(3)]
    public string Rule { get; set; } = string.Empty;
}

/// <summary>
/// "Judge this declaration of blockers for the pending question": answered with a <see cref="BlocksVerdictMessage"/>, to this
/// seat alone. Nothing in the game moves; the answer itself is still checked when it is given. For a
/// <see cref="DeclareBlockersPromptMessage"/> only.
/// </summary>
[MessagePackObject]
public sealed class CheckBlocksMessage : ClientMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    [Key(1)]
    public List<BlockMessage> Blocks { get; set; } = [];

    public override string Describe() => $"check {RequestId}: {Blocks.Count} blocks";
}

/// <summary>The rules' verdict on a declaration of blockers a <see cref="CheckBlocksMessage"/> asked about.</summary>
[MessagePackObject]
public sealed class BlocksVerdictMessage : ServerMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    /// <summary>The declaration the verdict is on, as it was asked.</summary>
    [Key(1)]
    public List<BlockMessage> Blocks { get; set; } = [];

    [Key(2)]
    public BlockVerdictView Verdict { get; set; } = new();

    public override string Describe() => $"blocks verdict {RequestId}: {(Verdict.Complete ? "complete" : "incomplete")}, {Verdict.Addable.Count} addable";
}

/// <summary>
/// This client chooses the target of a triggered ability it controls, as the ability goes on the stack (CR 603.3d), or of
/// one chosen mode of a modal spell it is casting, when <see cref="Ability"/> is the card in its hand.
/// </summary>
[MessagePackObject]
public sealed class ChooseTargetPromptMessage : PromptPayload, ISelectionPrompt
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The ability, already on the stack; or the modal spell being cast, still in hand.</summary>
    [Key(1)]
    public ViewHandle Ability { get; set; }

    /// <summary>The permanent whose ability it is. None if it has already left the battlefield.</summary>
    [Key(2)]
    public ViewHandle Source { get; set; }

    [Key(3)]
    public string Text { get; set; } = string.Empty;

    /// <summary>Every legal target. Answer with one of these, or none when <see cref="Optional"/>.</summary>
    [Key(4)]
    public List<TargetMessage> Candidates { get; set; } = [];

    /// <summary>"Up to one target": answering with no target is allowed.</summary>
    [Key(5)]
    public bool Optional { get; set; }

    /// <summary>
    /// How many targets to choose: one for almost everything, and more for a slot counted by X, as Candelabra of
    /// Tawnos' "Untap X target lands". Never more than <see cref="Candidates"/> offers.
    /// </summary>
    [Key(6)]
    public int Count { get; set; } = 1;

    /// <summary>
    /// Whether these are targets (CR 115.1) or things merely chosen as an effect resolves (CR 608.2d), as Comet,
    /// Stellar Pup's "to a creature or player" chooses. The answer is the same shape either way; only what the dock
    /// calls it differs, because a player told "target" would read the wrong rule off the screen.
    /// </summary>
    [Key(7)]
    public bool Targeted { get; set; } = true;

    /// <summary>
    /// The fewest targets a printed range takes — "one or two target creatures" is a <see cref="Count"/> of two and a
    /// minimum of one (CR 601.2c). Zero when there is no range: exactly <see cref="Count"/>, or none where <see cref="Optional"/>.
    /// </summary>
    [Key(8)]
    public int Minimum { get; set; }

    /// <summary>
    /// "Any number of" rather than "up to <see cref="Count"/>": every candidate or none may be chosen, as proliferate's
    /// "choose any number of permanents and/or players" (CR 701.34a). Comes with <see cref="Optional"/> and a
    /// <see cref="Count"/> of every candidate; only the dock's wording differs.
    /// </summary>
    [Key(9)]
    public bool AnyNumber { get; set; }

    /// <summary>
    /// The opponent whose slot this is, when the ability has one slot for each opponent (The Theorist's "for each opponent, return
    /// up to one target ..."): a seat, or zero. <see cref="Text"/> then names where it goes with "{player}".
    /// </summary>
    [Key(10)]
    public int ForPlayer { get; set; }

    /// <summary>
    /// "Any number of target creature cards with total mana value X or less" (CR 601.2c, 202.3): the most the chosen targets'
    /// <see cref="ManaValues"/> may add up to, as Fire Lord Sozin's reflexive ability asks. Null when the slot asks nothing of
    /// them. <c>Cantrip.Core.Decisions.TotalManaValue.Within</c> is the rule; the engine refuses a choice over it.
    /// </summary>
    [Key(11)]
    public int? MaximumTotalManaValue { get; set; }

    /// <summary>The mana value of each of <see cref="Candidates"/>, index for index, when <see cref="MaximumTotalManaValue"/> is set.</summary>
    [Key(12)]
    public List<int> ManaValues { get; set; } = [];

    /// <summary>
    /// What the rule of the slot's targets together asks, each in the words a caption puts after the noun ("with different
    /// mana values", "with mana value 3 or less"); empty when it asks nothing.
    /// </summary>
    [Key(13)]
    public List<string> Words { get; set; } = [];

    /// <summary>
    /// For a slot whose targets together are judged ("any number of target creature cards that each have a different mana
    /// value X or less", CR 601.2c): the rule's verdict on none chosen - whether that is an answer, and which object
    /// candidates may be picked first. Every other selection of the objects among the targets is asked of the host
    /// (<see cref="CheckSelectionMessage"/>) as for a choice of cards, and <c>GameClient.Answer</c> settles an answer with it.
    /// Null for a slot that asks nothing of its targets together.
    /// </summary>
    [Key(14)]
    public SelectionVerdictView? Verdict { get; set; }

    /// <summary>The object candidates, in order: what the rule of the targets together judges (a player is no card to it).</summary>
    IReadOnlyList<ViewHandle> ISelectionPrompt.Candidates => [.. Candidates.Where(candidate => !candidate.IsPlayer).Select(candidate => candidate.Object)];

    CountKindView ISelectionPrompt.CountKind =>
        AnyNumber ? CountKindView.AnyNumber : Optional || Minimum > 0 ? CountKindView.UpTo : CountKindView.Exactly;

    SelectionVerdictView ISelectionPrompt.Verdict => Verdict ?? new SelectionVerdictView { Complete = true };

    bool ISelectionPrompt.Judged => Verdict is not null;

    /// <summary>The total mana value of <paramref name="chosen"/>, each read at its place among <see cref="Candidates"/>.</summary>
    public int TotalManaValueOf(IEnumerable<TargetMessage> chosen) =>
        TotalManaValue.Of(Keys(), ManaValues, chosen.Select(Key));

    /// <summary>
    /// Whether <paramref name="candidate"/> can join <paramref name="chosen"/> and keep within <see cref="MaximumTotalManaValue"/>:
    /// always, without one, and for a target already chosen (choosing it again puts it back).
    /// </summary>
    public bool Fits(IReadOnlyCollection<TargetMessage> chosen, TargetMessage candidate) =>
        TotalManaValue.Fits(Keys(), ManaValues, [.. chosen.Select(Key)], Key(candidate), MaximumTotalManaValue);

    private List<(int, int)> Keys() => [.. Candidates.Select(Key)];

    private static (int, int) Key(TargetMessage target) => (target.Player, target.Object.Value);

    /// <summary>
    /// No target where that is allowed, and otherwise the first candidates the server offered; with a total mana value, the
    /// first that fit together within it; with another rule of the targets together, the first that may be picked first, as
    /// many as it takes (one asking more of them together is settled with the host as they are picked).
    /// </summary>
    public override AnswerPayload Default() =>
        new TargetsChosenAnswer
        {
            Targets = Optional ? []
                : MaximumTotalManaValue is { } most
                    ? TotalManaValue.FirstFitting(Candidates, ManaValues, Count, most)
                : Verdict is { } verdict
                    ? [.. Candidates.Where(candidate => candidate.IsPlayer || verdict.Addable.Contains(candidate.Object)).Take(Count)]
                    : [.. Candidates.Take(Count)],
        };
}

/// <summary>
/// This client chooses the targets of one slot marked "divided" and, together with them, how much of a total each
/// gets (CR 601.2d): "damage divided as you choose among any number of target creatures". Answered with a
/// <see cref="TargetsDividedAnswer"/>.
/// <para>
/// Not the same question as <see cref="AssignCombatDamagePromptMessage"/>, whose recipients are already fixed
/// blockers or attackers and only their shares are being asked for. Here the recipients themselves are the choice.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ChooseDividedTargetsPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell being cast, or the ability being activated or put on the stack.</summary>
    [Key(1)]
    public ViewHandle Ability { get; set; }

    /// <summary>The permanent whose ability it is, or the card being cast.</summary>
    [Key(2)]
    public ViewHandle Source { get; set; }

    [Key(3)]
    public string Text { get; set; } = string.Empty;

    /// <summary>Every legal target this slot could take. At most this many may be chosen.</summary>
    [Key(4)]
    public List<TargetMessage> Candidates { get; set; } = [];

    /// <summary>The total to divide among however many of <see cref="Candidates"/> are chosen.</summary>
    [Key(5)]
    public int Total { get; set; }

    /// <summary>Maximum chosen targets, or zero when only the total limits them.</summary>
    [Key(6)]
    public int MaxTargets { get; set; }

    /// <summary>
    /// Whether the slot reads "up to" (CR 601.2c): then choosing no target at all, and dividing nothing, is legal.
    /// </summary>
    [Key(7)]
    public bool Optional { get; set; }

    /// <summary>The whole total on the first candidate, or nothing when there is no total to divide.</summary>
    public override AnswerPayload Default() => new TargetsDividedAnswer
    {
        Shares = Total > 0 && Candidates.Count > 0
            ? [new TargetShareMessage { Target = Candidates[0], Amount = Total }]
            : [],
    };
}

/// <summary>One chosen target's own share of a divided total (CR 601.2d).</summary>
[MessagePackObject]
public sealed class TargetShareMessage
{
    [Key(0)]
    public TargetMessage Target { get; set; } = new();

    [Key(1)]
    public int Amount { get; set; }
}

/// <summary>
/// This client decides whether to do something a spell or ability says it "may" do (CR 608.2d), such as Path to Exile's
/// "its controller may search their library". Answered with a <see cref="MayChosenAnswer"/>.
/// </summary>
[MessagePackObject]
public sealed class MayPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell or ability on the stack that asks, or for <c>PayLife</c> the land about to enter.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>
    /// What would be done, as the effect's verb: <c>SearchLibrary</c>; or <c>PayLife</c>, a shockland's "As this land
    /// enters, you may pay 2 life. If you don't, it enters tapped."
    /// </summary>
    [Key(2)]
    public string Verb { get; set; } = string.Empty;

    /// <summary>How much, for a verb with an amount: the life <c>PayLife</c> pays. Otherwise 0.</summary>
    [Key(3)]
    public int Amount { get; set; }

    /// <summary>
    /// The card's own question, for a "you may" it words itself: after the source's name and a colon ("pay 2 life?"), or
    /// with <c>{source}</c> where the name goes. Empty for one the client words from <see cref="Verb"/>.
    /// </summary>
    [Key(4)]
    public string Question { get; set; } = string.Empty;

    /// <summary>What the choice means beyond its buttons ("If you don't, it enters tapped"), with a worded question.</summary>
    [Key(5)]
    public string Detail { get; set; } = string.Empty;

    /// <summary>The accepting button's caption, named for what it does ("Pay 2 life"), with a worded question.</summary>
    [Key(6)]
    public string Yes { get; set; } = string.Empty;

    /// <summary>The declining button's caption ("Enters tapped"), with a worded question.</summary>
    [Key(7)]
    public string No { get; set; } = string.Empty;

    /// <summary>Neither answer is the default, so passing through prompts never pays or risks it by accident.</summary>
    [Key(8)]
    public bool NoDefault { get; set; }

    /// <summary>Not doing it: "you may" never has to be taken up.</summary>
    public override AnswerPayload Default() => new MayChosenAnswer { Accept = false };
}

/// <summary>
/// This client chooses a player, without targeting them, as a spell or ability resolves (CR 608.2d): Baleful Mastery's
/// "an opponent draws a card". Answered with a <see cref="PlayerChosenAnswer"/> naming one of <see cref="Candidates"/>.
/// </summary>
[MessagePackObject]
public sealed class ChoosePlayerPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell or ability on the stack that asks.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>The seats that may be chosen, in turn order. Always two or more.</summary>
    [Key(2)]
    public List<int> Candidates { get; set; } = [];

    /// <summary>What the chosen player will do, as the effect's verb: <c>Draw</c>.</summary>
    [Key(3)]
    public string Verb { get; set; } = string.Empty;

    /// <summary>How much, for a verb with an amount: the cards <c>Draw</c> draws. Otherwise 0.</summary>
    [Key(4)]
    public int Amount { get; set; }

    /// <summary>
    /// The object the choice is for, when the effect treats several one at a time and chooses a player for each (Dack Fayden,
    /// Helping Hand's <c>GainControlOf</c>: the permanent the chosen opponent gains control of). No handle otherwise.
    /// </summary>
    [Key(5)]
    public ViewHandle Subject { get; set; }

    /// <summary>The first candidate in turn order.</summary>
    public override AnswerPayload Default() => new PlayerChosenAnswer { Player = Candidates.Count > 0 ? Candidates[0] : 0 };
}

/// <summary>
/// This client, chosen at random, decides who takes the first turn (CR 103.1), before any hand is dealt: itself to play
/// first, or in a two-player game the other seat to draw first. Answered with a <see cref="PlayerChosenAnswer"/> naming
/// one of <see cref="Candidates"/>.
/// </summary>
[MessagePackObject]
public sealed class ChooseStartingPlayerPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>Every seat, in turn order.</summary>
    [Key(1)]
    public List<int> Candidates { get; set; } = [];

    /// <summary>Playing first.</summary>
    public override AnswerPayload Default() => new PlayerChosenAnswer { Player = Player };
}

/// <summary>
/// This client keeps its hand or takes a mulligan (CR 103.5). Answered with a <see cref="MayChosenAnswer"/>: accepting
/// takes the mulligan. Keeping with mulligans taken is followed by a <see cref="ChooseCardsPromptMessage"/> for
/// <see cref="CardChoicePurposeView.PutOnBottom"/>.
/// </summary>
[MessagePackObject]
public sealed class MulliganPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>Mulligans taken so far: zero for the hand first dealt.</summary>
    [Key(1)]
    public int Mulligans { get; set; }

    /// <summary>How many cards keeping this hand keeps, once the mulligans' cards are put on the bottom.</summary>
    [Key(2)]
    public int Keeps { get; set; }

    /// <summary>Keeping.</summary>
    public override AnswerPayload Default() => new MayChosenAnswer { Accept = false };
}

/// <summary>
/// This client removes <see cref="Total"/> counters, of any kinds, from among permanents (CR 122.1): as part of a cost
/// (CR 118.3), "by removing three counters from among creatures you control" (Dawnhand Dissident), or as an effect resolves.
/// Each of <see cref="Pools"/> is the counters of one kind on one permanent, with how many there are; any of them may be
/// taken, adding up to the total - or, with <see cref="UpTo"/>, an effect's "remove up to three counters from target
/// creature" (Heartless Act, CR 608.2d), to any number from none to the total. Answered with a
/// <see cref="CountersRemovedAnswer"/>.
/// </summary>
[MessagePackObject]
public sealed class RemoveCountersPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell or ability: the one being cast or activated whose cost this is, or the one resolving.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>How many counters come off in all: with <see cref="UpTo"/>, the most that may.</summary>
    [Key(2)]
    public int Total { get; set; }

    [Key(3)]
    public List<CounterPoolMessage> Pools { get; set; } = [];

    /// <summary>"Up to" <see cref="Total"/> (CR 608.2d): any number of counters from none to the total may come off.</summary>
    [Key(4)]
    public bool UpTo { get; set; }

    /// <summary>As many as there are from each pool in turn, the first offered first, until the total is reached.</summary>
    public override AnswerPayload Default()
    {
        int left = Math.Max(0, Total);
        List<CounterPoolMessage> removals = [];
        foreach (CounterPoolMessage pool in Pools)
        {
            int n = Math.Min(pool.Count, left);
            if (n > 0)
            {
                removals.Add(new CounterPoolMessage { Object = pool.Object, Counter = pool.Counter, Count = n });
                left -= n;
            }
        }

        return new CountersRemovedAnswer { Removals = removals };
    }
}

/// <summary>
/// Counters of one kind on one permanent: in a <see cref="RemoveCountersPromptMessage"/> how many it has, in a
/// <see cref="CountersRemovedAnswer"/> how many come off. <see cref="Counter"/> names the kind as <see cref="CounterView"/>
/// does (<c>+1/+1</c>, <c>stun</c>).
/// </summary>
[MessagePackObject]
public sealed class CounterPoolMessage
{
    [Key(0)]
    public ViewHandle Object { get; set; }

    [Key(1)]
    public string Counter { get; set; } = string.Empty;

    [Key(2)]
    public int Count { get; set; }
}

/// <summary>
/// This client chooses one or more colors as a spell or ability resolves, or as a permanent enters (CR 202.2b):
/// Coalition Relic's "add one mana of any color for each charge counter removed this way". Answered with a
/// <see cref="ColorsChosenAnswer"/> naming <see cref="Count"/> of <see cref="Choices"/>.
/// </summary>
[MessagePackObject]
public sealed class ChooseColorsPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell, ability or permanent that asks.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>The colors that may be chosen, in WUBRG order. Never empty.</summary>
    [Key(2)]
    public List<ColorView> Choices { get; set; } = [];

    /// <summary>How many colors to choose. One for "choose a color".</summary>
    [Key(3)]
    public int Count { get; set; } = 1;

    /// <summary>Whether the same color may be chosen more than once.</summary>
    [Key(4)]
    public bool MayRepeat { get; set; }

    /// <summary>What the colors are for, as the effect's verb: <c>AddMana</c>.</summary>
    [Key(5)]
    public string Verb { get; set; } = string.Empty;

    /// <summary>The first color on offer, as many times as are wanted.</summary>
    public override AnswerPayload Default() => new ColorsChosenAnswer
    {
        Colors = MayRepeat
            ? [.. Enumerable.Repeat(Choices.Count > 0 ? Choices[0] : ColorView.White, Count)]
            : [.. Choices.Take(Count)],
    };
}

/// <summary>
/// This client chooses a card name (CR 201.4): as a permanent enters (CR 614.12a), as Anointed Peacekeeper's "As this
/// creature enters, look at an opponent's hand, then choose any card name", or as a spell or ability resolves, as
/// Petrified Hamlet's "choose a land card name". Answered with a
/// <see cref="CardNamedAnswer"/> naming one of <see cref="Names"/>.
/// </summary>
[MessagePackObject]
public sealed class NameCardPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The card about to enter, or the source of the spell or ability asking.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>Whose hand is being looked at, or zero for a name chosen without looking.</summary>
    [Key(2)]
    public int Owner { get; set; }

    /// <summary>That hand, in hand order, shown to this client alone.</summary>
    [Key(3)]
    public List<ViewHandle> Hand { get; set; } = [];

    /// <summary>Every name that may be chosen, sorted: each card the game knows.</summary>
    [Key(4)]
    public List<string> Names { get; set; } = [];

    /// <summary>A name worth choosing: the costliest nonland card in the hand looked at, or the first name there is.</summary>
    [Key(5)]
    public string Suggestion { get; set; } = string.Empty;

    /// <summary>
    /// What the name must be the name of, in the card's own words (CR 201.4a): "a card name", "a nonland card name" (Tamiyo,
    /// Collector of Tales), "a land card name" (Petrified Hamlet), "a card name other than a basic land card name"
    /// (Necromentia). <see cref="Names"/> already holds only those.
    /// </summary>
    [Key(6)]
    public string Kind { get; set; } = string.Empty;

    /// <summary>The suggestion, which is always one of <see cref="Names"/>.</summary>
    public override AnswerPayload Default() => new CardNamedAnswer { Name = Suggestion };
}

/// <summary>
/// Two or more replacement or prevention effects are trying to modify the same event, and this client chooses which
/// applies first (CR 616.1).
/// <para>
/// Sources and hints run in step: the client shows each source card with what it would do, and answers with the index
/// of the one to apply first. Every one of them is a legal answer.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class ChooseReplacementPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The object the event is about, or no handle for an event about no one object.</summary>
    [Key(1)]
    public ViewHandle Subject { get; set; }

    /// <summary>What the event is, in words.</summary>
    [Key(2)]
    public string Event { get; set; } = string.Empty;

    /// <summary>The source of each applicable effect, in the order they may be chosen.</summary>
    [Key(3)]
    public List<ViewHandle> Sources { get; set; } = [];

    /// <summary>What each of those effects would do, in words. Runs in step with <see cref="Sources"/>.</summary>
    [Key(4)]
    public List<string> Hints { get; set; } = [];

    /// <summary>
    /// Each source's name, in step with <see cref="Sources"/>: a source that has left (Ral Zarek, gone to the graveyard as
    /// its skip was made) may have no handle in this seat's view, and is named by this. Empty for one the seat may not see.
    /// </summary>
    [Key(5)]
    public List<string> SourceNames { get; set; } = [];

    /// <summary>The first offered effect. Any choice is legal, so the first is as good an automatic answer as any.</summary>
    public override AnswerPayload Default() => new ReplacementChosenAnswer { Index = 0 };
}

/// <summary>
/// Several of this client's abilities triggered at once, and they choose the order the abilities go on the stack
/// (CR 603.3b). The last one named goes on last and therefore resolves first. Answered with a
/// <see cref="TriggersOrderedAnswer"/>.
/// </summary>
[MessagePackObject]
public sealed class OrderTriggersPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The source of each ability, in the order they triggered.</summary>
    [Key(1)]
    public List<ViewHandle> Sources { get; set; } = [];

    /// <summary>Each ability as it reads. Runs in step with <see cref="Sources"/>.</summary>
    [Key(2)]
    public List<string> Texts { get; set; } = [];

    /// <summary>The order they triggered in. Any order is legal, so this needs no thought to be correct.</summary>
    public override AnswerPayload Default()
    {
        var order = new TriggersOrderedAnswer();

        for (int i = 0; i < Sources.Count; i++)
        {
            order.Order.Add(i);
        }

        return order;
    }
}

/// <summary>
/// This client may pay a cost to stop something happening (CR 118.12), such as Mana Leak's {3}.
/// <para>
/// Mana abilities are activated from here exactly as they are while paying for a spell. Covering the cost does not pay
/// it: paying is its own option, offered once the pool covers the cost, and declining is always offered.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class UnlessPaysPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>The spell asking, such as Mana Leak.</summary>
    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>What paying protects, such as the spell that would be countered.</summary>
    [Key(2)]
    public ViewHandle Subject { get; set; }

    /// <summary>The cost, in oracle notation.</summary>
    [Key(3)]
    public string Total { get; set; } = string.Empty;

    /// <summary>What the pool does not cover, in oracle notation. <c>{0}</c> when paying is possible now.</summary>
    [Key(4)]
    public string Remaining { get; set; } = string.Empty;

    [Key(5)]
    public int RemainingGeneric { get; set; }

    [Key(6)]
    public List<ColorView> RemainingColored { get; set; } = [];

    /// <summary>Mana abilities, undo, pay (when covered), and decline. Never empty.</summary>
    [Key(7)]
    public List<ActionOptionMessage> Options { get; set; } = [];

    /// <summary>Hybrid and Phyrexian symbols still owed (CR 107.4e-f), each payable another way besides its colors.</summary>
    [Key(8)]
    public List<HybridSymbolView> RemainingHybrid { get; set; } = [];

    /// <summary>
    /// What paying does, in card vocabulary, which the bots read it by: "Countered" for "unless pays" (paying keeps
    /// <see cref="Subject"/> from being countered), "Sacrificed" likewise for a permanent, and for "you may pay. If you
    /// do" what paying buys - "CopySpell", "UntapSource", "ReturnToHand", "ReturnToBattlefield", "ReturnPermanentCard"
    /// (a card chosen once paid, its mana value at most <see cref="Amount"/>), "DestroyCreatureOrPlaneswalker" (targets
    /// chosen once paid, among an opponent's creatures and planeswalkers), "CreateCopyToken", "Transform",
    /// "PutOntoBattlefield", "LookAgain", "EnterUntapped".
    /// </summary>
    [Key(9)]
    public string Verb { get; set; } = string.Empty;

    /// <summary>The life the cost takes besides its mana (CR 119.4): "{W}{B} and 2 life" is a <see cref="Total"/> of {W}{B} and 2.</summary>
    [Key(10)]
    public int Life { get; set; }

    /// <summary>The life total no longer covers <see cref="Life"/>: a mana ability made for the payment cost it, and only declining is left.</summary>
    [Key(16)]
    public bool LifeShort { get; set; }

    /// <summary>An amount the verb carries: for "ReturnPermanentCard", the greatest mana value the card may have.</summary>
    [Key(11)]
    public int Amount { get; set; }

    /// <summary>
    /// The card's words for the payment, after the source's name: <c>{cost}</c> and <c>{subject}</c> stand where the cost
    /// (<see cref="CostText"/>) and the subject's name go, here and in <see cref="Detail"/>, <see cref="Yes"/> and <see cref="No"/>.
    /// </summary>
    [Key(12)]
    public string Question { get; set; } = string.Empty;

    /// <summary>What the payment means beyond its buttons ("If you don't, it enters tapped"). May be empty.</summary>
    [Key(13)]
    public string Detail { get; set; } = string.Empty;

    /// <summary>The button that pays, named for what it does.</summary>
    [Key(14)]
    public string Yes { get; set; } = string.Empty;

    /// <summary>The button that declines, named for what it does.</summary>
    [Key(15)]
    public string No { get; set; } = string.Empty;

    /// <summary>
    /// "You may pay {X}" with X left to the payer (CR 107.3f), as Fire Lord Sozin's: X is chosen as it is paid. <see cref="X"/>
    /// is the X so far and <see cref="Total"/> the cost at it; IncreaseX and DecreaseX among the <see cref="Options"/> change
    /// it, and paying pays that X.
    /// </summary>
    [Key(17)]
    public bool ChoosingX { get; set; }

    /// <summary>The X chosen so far, when <see cref="ChoosingX"/>.</summary>
    [Key(18)]
    public int X { get; set; }

    /// <summary>
    /// "You may pay {G} up to three times", "any number of times" (CR 603.12a), as Tranquil Frillback's: the number of times is
    /// chosen as it is paid. <see cref="X"/> is the number so far (from 1), <see cref="Total"/> the cost that many times over;
    /// IncreaseX and DecreaseX among the <see cref="Options"/> change it within 1 to <see cref="MostTimes"/>, and paying pays
    /// it that many times.
    /// </summary>
    [Key(19)]
    public bool ChoosingTimes { get; set; }

    /// <summary>The most times it may be paid, when <see cref="ChoosingTimes"/>; 0 for any number of times.</summary>
    [Key(20)]
    public int MostTimes { get; set; }

    /// <summary>The whole cost in words: "{W}{B} and 2 life", "{3}", "2 life".</summary>
    public string CostText()
    {
        bool mana = Total.Length > 0 && Total != "{0}";
        return Life <= 0 ? (mana ? Total : "{0}") : mana ? $"{Total} and {Life} life" : $"{Life} life";
    }

    /// <summary>One of the card's texts with the cost and the subject's name put in.</summary>
    public string Words(string text, string subject) =>
        (text ?? string.Empty).Replace("{cost}", CostText(), StringComparison.Ordinal).Replace("{subject}", subject, StringComparison.Ordinal);

    /// <summary>Declining, which is always among the options.</summary>
    public override AnswerPayload Default() => new TakeActionAnswer { Kind = ActionKindView.DeclineCost };
}

/// <summary>
/// This client has priority and may act or pass.
/// <para>
/// Options are prototypes the client fills in, never a flat enumeration of concrete actions: an X spell with
/// modes and divided damage would be millions of tuples. The client composes a candidate and the server
/// validates it, which is also why announcing a spell never makes the engine suspend mid-cast.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class PriorityPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public List<ActionOptionMessage> Options { get; set; } = [];

    /// <summary>Passing, which is always legal (CR 117.3d).</summary>
    public override AnswerPayload Default() => new PassAnswer();
}

[MessagePackObject]
public sealed class ActionOptionMessage
{
    [Key(0)]
    public ActionKindView Kind { get; set; }

    [Key(1)]
    public ViewHandle Source { get; set; }

    /// <summary>
    /// Which of the source's abilities, for an activation. Indexes the computed ability list. For a cast, which way to
    /// pay: 0 for the mana cost, n for the card's nth alternative cost (CR 118.9), described in <see cref="Description"/>.
    /// </summary>
    [Key(2)]
    public int AbilityIndex { get; set; }

    /// <summary>How many targets this action needs, zero for none.</summary>
    [Key(3)]
    public int TargetsRequired { get; set; }

    /// <summary>What the target could legally be. Advice the client picks from; the server re-checks the choice.</summary>
    [Key(4)]
    public List<TargetMessage> TargetCandidates { get; set; } = [];

    /// <summary>
    /// The mana an activation would add, for a mana ability. Advice: it lets a client paying a cost tell which land
    /// helps without knowing what a Forest is.
    /// </summary>
    [Key(5)]
    public List<ManaTypeView> Produces { get; set; } = [];

    /// <summary>
    /// For a cast: whether the pool plus the player's untapped mana sources could pay for it. Advice only — the spell
    /// may be announced regardless and taken back if it cannot be paid for — so a client that would rather not try,
    /// such as a bot, knows not to.
    /// </summary>
    [Key(6)]
    public bool Affordable { get; set; }

    /// <summary>
    /// For an activation: mana of a single color the ability adds on top of <see cref="Produces"/>. When more than zero,
    /// the answer has to name a color, as activating Black Lotus does.
    /// </summary>
    [Key(7)]
    public int ProducesOfAnyOneColor { get; set; }

    /// <summary>For a cast: whether the cost has {X}, so the answer carries a chosen X.</summary>
    [Key(8)]
    public bool HasX { get; set; }

    /// <summary>For a cast with {X}: the largest X that could be paid for, or -1 if none could. Advice only.</summary>
    [Key(9)]
    public int MaxX { get; set; }

    /// <summary>
    /// For an activation: how the ability reads, cost and all, such as "{T}, Sacrifice Wasteland: Destroy target nonbasic
    /// land." For showing on a menu, and no client reads a rule out of it. The tactical seat reads it as a player reads
    /// the menu, since it is all that says which ability this is; what it makes of it only chooses among what is offered.
    /// </summary>
    [Key(11)]
    public string Description { get; set; } = string.Empty;

    /// <summary>For an activation: life it costs.</summary>
    [Key(12)]
    public int LifeCost { get; set; }

    /// <summary>For a mana ability: damage it deals to its controller as it resolves.</summary>
    [Key(13)]
    public int DamageToController { get; set; }

    /// <summary>
    /// For an activation with <see cref="ProducesOfAnyOneColor"/>: the colors the answer may name, all five for Black
    /// Lotus and two for a Talisman. Naming any other is refused.
    /// </summary>
    [Key(14)]
    public List<ColorView> ColorChoices { get; set; } = [];

    /// <summary>
    /// For a mana ability: its cost as rules text, such as "{T}, Pay 1 life". Lets a client write one menu row per color
    /// the ability could add. For showing; never parsed.
    /// </summary>
    [Key(15)]
    public string CostText { get; set; } = string.Empty;

    /// <summary>
    /// "Up to one target" (CR 601.2c): the action may be taken aiming at nothing even while
    /// <see cref="TargetCandidates"/> offers something, so a client aiming it offers declining as well.
    /// </summary>
    [Key(16)]
    public bool TargetsOptional { get; set; }

    /// <summary>
    /// For an offer to put pool mana towards a spell: what that mana may be spent on. Its type is in <see cref="Produces"/>.
    /// For a mana ability: what the mana it adds may be spent on.
    /// </summary>
    [Key(17)]
    public ManaSpendFilterView Filter { get; set; }

    /// <summary>What spending that mana does to whatever it pays for, alongside <see cref="Filter"/>.</summary>
    [Key(18)]
    public ManaSpendEffectView Effect { get; set; }

    /// <summary>
    /// For a cast: the name of the spell this would cast, when that is not the source card's own name — an adventurer
    /// card cast as its Adventure (CR 715.3) casts "Battle Display", not Embereth Shieldbreaker. Empty when the spell
    /// is the card, which is the usual case. For showing on a menu; never parsed.
    /// </summary>
    [Key(19)]
    public string SpellName { get; set; } = string.Empty;

    /// <summary>
    /// For an activation: mana the ability adds whose colors are each chosen on their own (CR 106.1b), as Orcish
    /// Lumberjack's "three mana in any combination of {R} and/or {G}". When more than zero, the answer names that many
    /// colors out of <see cref="ColorChoices"/>, repeats and all — which is what one color for all of them cannot say.
    /// </summary>
    [Key(20)]
    public int ProducesOfAnyColorEach { get; set; }

    /// <summary>
    /// For an activation whose cost sacrifices a permanent its controller chooses (CR 118.3, 701.21a), as Orcish
    /// Lumberjack's "Sacrifice a Forest": the permanents that could pay it. When non-empty, the answer names one.
    /// </summary>
    [Key(21)]
    public List<ViewHandle> SacrificeCandidates { get; set; } = [];

    /// <summary>The controlled permanent this option taps to pay its mana ability cost.</summary>
    [Key(22)]
    public ViewHandle? Tap { get; set; }

    /// <summary>
    /// For a tap-to-pay or exile-to-pay option paying {1} of a cost's generic mana (waterbend, improvise, delve): how
    /// many more {1} its way of paying can still pay, however many candidates are offered for it. Zero for convoke.
    /// No mana ability is offered once one of these has paid (CR 601.2g-h), so a seat makes the rest in mana first.
    /// </summary>
    [Key(23)]
    public int PaysUpTo { get; set; }

    /// <summary>With <see cref="PaysUpTo"/>: which of the cost's ways of paying generic mana this candidate pays.</summary>
    [Key(24)]
    public int Alternative { get; set; }

    /// <summary>For a mana ability: life its controller gains as it resolves, as The Great Henge's.</summary>
    [Key(25)]
    public int LifeGained { get; set; }

    /// <summary>
    /// For a mana ability the player has of their own (CR 611.2a, 605.1a), which comes with no <see cref="Source"/>:
    /// the name of what gave it to them, "Channel", whatever has become of that since. Empty for anything else. For
    /// showing; never parsed.
    /// </summary>
    [Key(26)]
    public string GrantedBy { get; set; } = string.Empty;

    /// <summary>
    /// For a mana ability offered while a cost is paid: the mana it adds is restricted so that none of it may pay this
    /// cost (CR 106.6) - Sunken Citadel's two mana for a spell, Mishra's Workshop's for a creature. It may still be
    /// activated (CR 601.2g); its mana only waits in the pool. <see cref="Filter"/> says what that mana is for.
    /// </summary>
    [Key(27)]
    public bool PaysNothing { get; set; }

    /// <summary>The subtypes <see cref="Filter"/> names, as <see cref="RestrictedManaView.SpendSubtypes"/>.</summary>
    [Key(28)]
    public List<string> SpendSubtypes { get; set; } = [];

    /// <summary>The mana value <see cref="Filter"/> names, as <see cref="RestrictedManaView.SpendManaValue"/>.</summary>
    [Key(29)]
    public int SpendManaValue { get; set; }

    /// <summary>
    /// For a cast: a way a permanent gives that casts the spell without paying its mana cost (CR 118.9: Omniscience's "you
    /// may cast spells from your hand without paying their mana costs"), offered beside the ordinary cast. X is 0 (CR
    /// 107.3b); additional costs and cost increases are still paid, and <see cref="CostText"/> is what it comes to with the
    /// cost changes in play before any target: <c>{0}</c>, or <c>{1}</c> under Thalia.
    /// </summary>
    [Key(30)]
    public bool WithoutPayingManaCost { get; set; }

    /// <summary>
    /// For a cast or an activation: its cost returns tapped permanents, chosen once the mana is paid (CR 601.2g-h, 602.2b:
    /// web-slinging's "return a tapped creature you control", Urban Retreat's). One tapped for mana while paying may be one
    /// of them, and a payment that leaves too few tapped is taken back.
    /// </summary>
    [Key(31)]
    public bool TappedAfterMana { get; set; }
    /// <summary>Opaque option identity in this question; echo it when choosing this option.</summary>
    [Key(32)]
    public int OptionId { get; set; }

    /// <summary>
    /// For a cast: it casts the card face down, by morph or disguise (CR 702.37c, 702.168b, 708.4), as a 2/2 face-down creature
    /// spell with no name, text or mana cost (a disguised one with ward {2}) for <see cref="CostText"/>, {3}.
    /// <see cref="Description"/> is the keyword line ("Morph {3}{W}"). Its controller alone may look at it on the stack (CR 708.5).
    /// </summary>
    [Key(33)]
    public bool CastFaceDown { get; set; }

    /// <summary>
    /// For a cast: it casts the card prototyped (CR 702.160a, 718.3), with its prototype's mana cost (<see cref="CostText"/>),
    /// power, toughness and color; <see cref="Description"/> is the keyword line ("Prototype {2}{R} — 3/2").
    /// </summary>
    [Key(34)]
    public bool CastPrototyped { get; set; }
}

/// <summary>
/// This client has announced a spell and still owes part of its cost (CR 601.2g-h).
/// <para>
/// The client tapping a land here sends an activation exactly as it would from a priority prompt; the server applies
/// the pool the moment it covers the cost in one way only, the spell goes onto the stack, and the prompt goes away.
/// When the pool could cover it in more than one way — a different mana would be left behind, counting a restriction
/// as a difference — the options offer pool mana to put towards the cost instead, one per kind that would narrow the
/// ways down, and the spell is cast once one way is left. Stopping is always offered, and returns the game to the
/// moment before the spell was announced (CR 733.1). Nobody else is told about any of this until the spell is paid
/// for.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class PayCostsPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>
    /// The spell being paid for, on the stack already (CR 601.2a); or, for an ability, its source, wherever that is - the
    /// ability itself is on the stack (CR 602.2a).
    /// </summary>
    [Key(1)]
    public ViewHandle Spell { get; set; }

    /// <summary>The whole cost, in oracle notation.</summary>
    [Key(2)]
    public string Total { get; set; } = string.Empty;

    /// <summary>What the pool does not cover, in oracle notation. For display.</summary>
    [Key(3)]
    public string Remaining { get; set; } = string.Empty;

    /// <summary>Generic mana still owed, so a client can tell whether any mana at all would help.</summary>
    [Key(4)]
    public int RemainingGeneric { get; set; }

    /// <summary>Coloured symbols still owed, one entry per symbol, so a client can tell which land would help.</summary>
    [Key(5)]
    public List<ColorView> RemainingColored { get; set; } = [];

    /// <summary>
    /// What the player may do: mana abilities to activate, the last one to take back, and stopping; pool mana to put
    /// towards the cost, when the pool could pay it in more than one way and which is the player's to choose; and for
    /// a spell with {X}, raising and lowering X, and casting. Never empty.
    /// </summary>
    [Key(6)]
    public List<ActionOptionMessage> Options { get; set; } = [];

    /// <summary>Whether the cost has {X}, so X is being chosen here rather than the cost simply being covered.</summary>
    [Key(7)]
    public bool HasX { get; set; }

    /// <summary>X so far. Zero for a spell without {X}.</summary>
    [Key(8)]
    public int X { get; set; }

    /// <summary>
    /// Mana in the pool already put towards the spell, by type and restriction. Still in the pool until the spell is
    /// cast, so a client showing the pool can say how much of it is spoken for.
    /// </summary>
    [Key(9)]
    public List<RestrictedManaView> Committed { get; set; } = [];

    /// <summary>
    /// Hybrid and Phyrexian symbols still owed (CR 107.4e-f), one entry per symbol, each payable another way too. Apart from
    /// <see cref="RemainingColored"/> because either colour will do, and a client that read only that list would think
    /// a cost of <c>{U/R}</c> owed nothing and give up paying for it.
    /// </summary>
    [Key(10)]
    public List<HybridSymbolView> RemainingHybrid { get; set; } = [];

    /// <summary>
    /// The spell's kicker cost in oracle notation (CR 702.33), or empty for none. A variable kicker uses
    /// <see cref="X"/> as its chosen value and <see cref="KickerPaid"/> to distinguish paid zero from declining it.
    /// </summary>
    [Key(11)]
    public string Kicker { get; set; } = string.Empty;

    /// <summary>Whether the kicker is multikicker (CR 702.33c), payable any number of times rather than once.</summary>
    [Key(12)]
    public bool Multikicker { get; set; }

    /// <summary>
    /// Whether that optional additional cost is buyback (CR 702.27) rather than kicker: the same payment, so
    /// <see cref="Kicker"/> still carries the cost and <see cref="X"/> whether it has been paid, but a client says
    /// "buy back" rather than "kick".
    /// </summary>
    [Key(13)]
    public bool Buyback { get; set; }

    /// <summary>
    /// Whether this is the announcement rather than the payment (CR 601.2b): <see cref="X"/> — the value of X, or with a
    /// <see cref="Kicker"/> how many times it is kicked — is being chosen before any target, and nothing is paid yet. Its
    /// only options are raising and lowering it, announcing it, and giving up; the payment is asked afterwards.
    /// </summary>
    [Key(14)]
    public bool AnnouncesXOrKicks { get; set; }

    /// <summary>
    /// For the announcement: the largest value the pool and the player's untapped mana sources could pay for, or -1
    /// when not even zero could be; for an X paid in life, the most the player could pay and still have life left.
    /// Advice only, like <see cref="ActionOptionMessage.MaxX"/>.
    /// </summary>
    [Key(15)]
    public int MostAffordable { get; set; }

    /// <summary>
    /// For the announcement: whether X is paid in life ("pay X life", CR 119.4) rather than mana, so a client says
    /// "life" and a bot spends it with the care life deserves.
    /// </summary>
    [Key(16)]
    public bool XPaidInLife { get; set; }

    /// <summary>
    /// Colorless mana symbols {C} still owed (CR 107.4c), so a client can tell that a colored or generic-only source
    /// would not help — parallel to <see cref="RemainingGeneric"/>, but for the count that must be paid with
    /// colorless mana specifically rather than with anything.
    /// </summary>
    [Key(17)]
    public int RemainingColorless { get; set; }

    /// <summary>Whether the optional {X} kicker was chosen, including at X=0.</summary>
    [Key(18)]
    public bool KickerPaid { get; set; }

    /// <summary>Whether the kicker's cost contains {X}, instead of X counting kicker payments.</summary>
    [Key(19)]
    public bool VariableKicker { get; set; }

    /// <summary>
    /// Whether no mana may be spent on this cost at all ("You can't spend mana to cast this spell", CR 601.2h, 106.4):
    /// only convoke's taps and delve's exiles pay it. Mana abilities are still offered (CR 601.2g), but their mana pays
    /// nothing here, so a client neither asks for lands or pool mana nor makes any.
    /// </summary>
    [Key(20)]
    public bool SpendsNoMana { get; set; }

    /// <summary>
    /// The attacking creatures whose cost to attack this pays (CR 508.1h-j), when it is one: <see cref="Spell"/> is then
    /// the permanent whose static ability makes attacking cost something (the first, when several do), and giving up takes
    /// the declaration of attackers back, to be made again. Empty for any other payment.
    /// </summary>
    [Key(21)]
    public List<ViewHandle> Attackers { get; set; } = [];

    /// <summary>
    /// Announcing the X of a "−X" loyalty cost (CR 606.4, 107.3): X loyalty counters come off <see cref="Spell"/>, the
    /// permanent, and no mana is paid. <see cref="MostAffordable"/> is its loyalty.
    /// </summary>
    [Key(22)]
    public bool XRemovesLoyalty { get; set; }

    /// <summary>
    /// The life the total cost asks besides its mana (CR 119.4, 601.2h): an additional cost's "pay X life", or one a static
    /// ability adds ("spells your opponents cast that target this creature cost an additional 3 life to cast"). Paid with
    /// it once the mana is. Zero while X is announced.
    /// </summary>
    [Key(23)]
    public int Life { get; set; }

    /// <summary>
    /// Announcing how many permanents the spell's additional cost sacrifices (CR 601.2b): "you may sacrifice any number of
    /// nonland permanents. This spell costs {1} less to cast for each permanent sacrificed this way" (Rottenmouth Viper).
    /// Each takes this much generic mana off <see cref="Total"/>; <see cref="X"/> is the number so far, and
    /// <see cref="MostAffordable"/> how many could be. Which ones is asked once the mana is paid. Zero for any other
    /// announcement.
    /// </summary>
    [Key(24)]
    public int SacrificeReduces { get; set; }

    /// <summary>
    /// For the announcement: the largest value an <see cref="AnnounceAnswer"/> may announce (CR 601.2b) - the rules' limit,
    /// such as one for a kicker paid at most once, rather than what could be paid for, which is <see cref="MostAffordable"/>.
    /// Zero while paying.
    /// </summary>
    [Key(25)]
    public int MostAnnounced { get; set; }

    /// <summary>
    /// For the announcement: the smallest value an <see cref="AnnounceAnswer"/> may announce: 1 for an X that can't be 0
    /// (CR 107.3a, 601.2b), else 0. The announcement starts at <see cref="X"/>: this, or the X a reduction already announced
    /// pays for (the cards a March exiles from the hand, CR 601.2f). Zero while paying.
    /// </summary>
    [Key(26)]
    public int LeastAnnounced { get; set; }

    /// <summary>
    /// For an announcement of an X that is not paid in mana: what X governs, in words (casualty X's "the least power of the
    /// creature you sacrifice", CR 702.153a), which a client captions the announcement with in place of the mana wording.
    /// Empty for any other.
    /// </summary>
    [Key(27)]
    public string XGoverns { get; set; } = string.Empty;

    /// <summary>
    /// Paying for a spell with assist (CR 702.132a), the step this question is (<see cref="AssistSteps"/>): the chosen player's
    /// chance to activate mana abilities (<see cref="AssistSteps.HelperActivates"/>), then the caster's
    /// (<see cref="AssistSteps.CasterActivates"/>), each closed with <see cref="ActionKindView.ConfirmCasting"/>; the chosen
    /// player announcing how much of the generic mana they pay (<see cref="AssistSteps.HelperAnnounces"/>, at most
    /// <see cref="MostAnnounced"/>) and paying it from their pool (<see cref="AssistSteps.HelperPays"/>); and the caster paying
    /// the rest with no mana ability offered (<see cref="AssistSteps.CasterPays"/>). The chosen player's steps offer no giving
    /// up: the spell is not theirs. In the two windows <see cref="Remaining"/> is the whole of what is owed, not what the pool
    /// leaves. Empty for any other payment.
    /// </summary>
    [Key(28)]
    public string AssistStep { get; set; } = string.Empty;

    /// <summary>The mana other players have paid towards this spell as it is cast: the assisting player's (CR 702.132a).</summary>
    [Key(29)]
    public int Assisted { get; set; }

    /// <summary>
    /// Whether that optional additional cost is squad (CR 702.157a) rather than kicker: payable any number of times as
    /// multikicker is, but <see cref="Kicker"/> then carries its whole cost in words (mana and what is beside it, such as
    /// exiling cards from a graveyard), so a client says "pay squad", not "kick", and speaks of the limit
    /// (<see cref="MostAnnounced"/>) as times, not as mana.
    /// </summary>
    [Key(30)]
    public bool Squad { get; set; }

    /// <summary>
    /// Giving up on the spell (CR 733.1). A question that offers no giving up - the steps of a player assisting with another's
    /// spell (CR 702.132a) - closes their window or announces nothing more, or pays with the first pool mana offered: what they
    /// announced they must pay. The host's own default answer, the same way.
    /// </summary>
    public override AnswerPayload Default() => GiveUp();

    /// <summary><see cref="Default"/>, as the action it is.</summary>
    public TakeActionAnswer GiveUp()
    {
        if (!Options.Exists(option => option.Kind == ActionKindView.CancelCasting))
        {
            if (Options.Find(option => option.Kind == ActionKindView.ConfirmCasting) is { } confirm)
            {
                return TakeActionAnswer.Take(confirm);
            }

            if (Options.Find(option => option.Kind == ActionKindView.AssignMana) is { } mana)
            {
                TakeActionAnswer answer = TakeActionAnswer.Take(mana);
                answer.Mana = mana.Produces.Count > 0 ? mana.Produces[0] : null;
                return answer;
            }
        }

        return new TakeActionAnswer { Kind = ActionKindView.CancelCasting };
    }

    /// <summary>Whether this asks the player assisting with another player's spell (CR 702.132a), who may not give it up.</summary>
    [IgnoreMember]
    public bool Assisting => AssistStep is AssistSteps.HelperActivates or AssistSteps.HelperAnnounces or AssistSteps.HelperPays;
}

/// <summary>The steps of a payment with assist (CR 702.132a), as <see cref="PayCostsPromptMessage.AssistStep"/> names them.</summary>
public static class AssistSteps
{
    public const string HelperActivates = "HelperActivates";
    public const string CasterActivates = "CasterActivates";
    public const string HelperAnnounces = "HelperAnnounces";
    public const string HelperPays = "HelperPays";
    public const string CasterPays = "CasterPays";
}

/// <summary>
/// One symbol with more than one printed way to pay it: a two-color hybrid symbol, payable with either colour
/// (CR 107.4e), or a Phyrexian symbol, payable with its one colour or with life (CR 107.4f).
/// </summary>
[MessagePackObject]
public sealed class HybridSymbolView
{
    /// <summary>The colours one mana of which could pay this symbol: both, for hybrid; the one, for Phyrexian.</summary>
    [Key(0)]
    public List<ColorView> Colors { get; set; } = [];

    /// <summary>The life that pays this symbol instead of mana (CR 107.4f), or zero for a plain hybrid symbol.</summary>
    [Key(1)]
    public int LifeCost { get; set; }

    public bool Accepts(ColorView color) => Colors.Contains(color);
}

/// <summary>Something a spell may target: a player, or an object named by handle.</summary>
[MessagePackObject]
public sealed class TargetMessage
{
    /// <summary>Set when the target is a player; zero otherwise.</summary>
    [Key(0)]
    public int Player { get; set; }

    /// <summary>Set when the target is an object; none otherwise.</summary>
    [Key(1)]
    public ViewHandle Object { get; set; }

    [IgnoreMember]
    public bool IsPlayer => Player != 0;
}

/// <summary>The active player must discard down to their maximum hand size (CR 514.1).</summary>
[MessagePackObject]
public sealed class DiscardPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public int Count { get; set; }

    [Key(2)]
    public int MaximumHandSize { get; set; }

    [Key(3)]
    public List<ViewHandle> Hand { get; set; } = [];

    /// <summary>The front of the hand, which is as good a choice as any when nobody is choosing.</summary>
    public override AnswerPayload Default() => new CardsChosenAnswer { Cards = [.. Hand.Take(Count)] };
}

/// <summary>The active player declares attackers (CR 508.1).</summary>
[MessagePackObject]
public sealed class DeclareAttackersPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public List<ViewHandle> Eligible { get; set; } = [];

    /// <summary>
    /// Seats that may be attacked. A list rather than a single defender, because each attacker picks its own
    /// target in a multiplayer game (CR 508.1).
    /// </summary>
    [Key(2)]
    public List<int> AttackablePlayers { get; set; } = [];

    /// <summary>Planeswalkers an opponent controls, each of which an attacker may be sent at instead (CR 508.1b).</summary>
    [Key(3)]
    public List<ViewHandle> AttackablePlaneswalkers { get; set; } = [];

    /// <summary>
    /// Battles a defending player protects, each of which an attacker may be sent at instead (CR 310.9b, 508.1b): a
    /// Siege the attacking player controls among them, since its protector is their opponent (CR 310.12a).
    /// </summary>
    [Key(5)]
    public List<ViewHandle> AttackableBattles { get; set; } = [];

    /// <summary>
    /// The requirements on the declaration (CR 508.1d), one for each source, for showing the player: a goaded creature has
    /// two for each player who goaded it (CR 701.15b). The host judges how many a declaration obeys (<see cref="Verdict"/>).
    /// </summary>
    [Key(4)]
    public List<AttackRequirementMessage> Requirements { get; set; } = [];

    /// <summary>
    /// Pairs of an eligible creature and an offered seat, planeswalker or battle that may not be declared, because
    /// something says that one can't be attacked by that creature (CR 508.1c). A declaration naming one is refused. A
    /// seat, planeswalker or battle no eligible creature may attack is left out of the lists above instead.
    /// </summary>
    [Key(6)]
    public List<AttackMessage> Forbidden { get; set; } = [];

    /// <summary>
    /// What each attack that costs something to declare costs (CR 508.1d, 508.1h): "creatures can't attack you unless their
    /// controller pays {2} for each ...". The declaration's total is what its attacks' costs add up to, paid once it is
    /// declared (a payment prompt follows); a cost is never a requirement, so a creature that must attack need not attack
    /// where every attack open to it costs something. Attacks not listed cost nothing.
    /// </summary>
    [Key(7)]
    public List<AttackCostMessage> Costs { get; set; } = [];

    /// <summary>
    /// While <see cref="Costs"/> has any: the most mana the player could pay now (the pool and what their mana abilities
    /// add, counting creatures that attacking would tap), or -1 when that is not known. Advice only.
    /// </summary>
    [Key(8)]
    public int CanPay { get; set; }

    /// <summary>What <paramref name="attack"/> costs to declare, or null when it costs nothing.</summary>
    public AttackCostMessage? CostOf(AttackMessage attack) =>
        Costs.Find(cost => cost.Attacker == attack.Attacker
            && cost.TargetPermanent == attack.TargetPermanent
            && (!cost.TargetPermanent.IsNone || cost.TargetPlayer == attack.TargetPlayer));

    /// <summary>The mana value of what declaring <paramref name="attacks"/> costs in all (CR 508.1h).</summary>
    public int TotalCost(IEnumerable<AttackMessage> attacks) => attacks.Sum(attack => CostOf(attack)?.ManaValue ?? 0);

    /// <summary>
    /// What declaring <paramref name="attacks"/> costs in all, in oracle notation ("{4}"), or empty when nothing: the
    /// generic parts added up, any other symbols after them.
    /// </summary>
    public string TotalCostText(IEnumerable<AttackMessage> attacks)
    {
        List<AttackCostMessage> costs = [.. attacks.Select(CostOf).OfType<AttackCostMessage>()];
        if (costs.Count == 0)
        {
            return string.Empty;
        }

        int generic = costs.Sum(cost => cost.Generic);
        string rest = string.Concat(costs.Select(cost => cost.Generic > 0 && cost.Cost.StartsWith('{')
            ? cost.Cost[(cost.Cost.IndexOf('}') + 1)..]
            : cost.Cost));
        return (generic > 0 ? $"{{{generic}}}" : string.Empty) + rest;
    }

    /// <summary>Whether <paramref name="attack"/> is one of <see cref="Forbidden"/>'s pairs.</summary>
    public bool IsForbidden(AttackMessage attack) =>
        Forbidden.Any(refused => refused.Attacker == attack.Attacker
            && refused.TargetPermanent == attack.TargetPermanent
            && (!refused.TargetPermanent.IsNone || refused.TargetPlayer == attack.TargetPlayer));

    /// <summary>
    /// Every attack <paramref name="attacker"/> may be declared making, seats first: each offered seat, planeswalker
    /// and battle less the forbidden pairs.
    /// </summary>
    public List<AttackMessage> AttacksFor(ViewHandle attacker) =>
    [
        .. AttackablePlayers.Select(seat => new AttackMessage { Attacker = attacker, TargetPlayer = seat })
            .Concat(AttackablePlaneswalkers.Select(walker => new AttackMessage { Attacker = attacker, TargetPermanent = walker }))
            .Concat(AttackableBattles.Select(battle => new AttackMessage { Attacker = attacker, TargetPermanent = battle }))
            .Where(attack => !IsForbidden(attack)),
    ];

    /// <summary>How many of <see cref="Requirements"/> the declaration must obey (CR 508.1d), for showing the player.</summary>
    [Key(9)]
    public int MostRequirements { get; set; }

    /// <summary>A declaration that obeys that many, the engine's own; empty where nothing is required.</summary>
    [Key(10)]
    public List<AttackMessage> Suggested { get; set; } = [];

    /// <summary>
    /// The rules' verdict on declaring no attackers (CR 508.1a-d): whether that may be declared, why not, and the attacks
    /// that may be declared first. Every other declaration is judged by the host as it is made (<see cref="CheckAttacksMessage"/>).
    /// </summary>
    [Key(11)]
    public AttackVerdictView Verdict { get; set; } = new() { Complete = true };

    /// <summary>No attack, or where an effect requires attacks (CR 508.1d) the engine's own declaration, which obeys them.</summary>
    public override AnswerPayload Default() => new AttackersDeclaredAnswer { Attacks = [.. Suggested] };

    /// <summary>
    /// Whether the engine's own declaration (<see cref="Suggested"/>) has <paramref name="attacker"/> attack: some requirement
    /// on it is obeyed by attacking (CR 508.1d), as the host worked out. A suggestion, not a rule: whether a declaration
    /// leaving it out obeys enough requirements is the host's to judge (<see cref="Verdict"/>, <see cref="CheckAttacksMessage"/>),
    /// and an answer that does not is mended by <c>GameClient.Answer</c>.
    /// </summary>
    public bool IsRequired(ViewHandle attacker) => Suggested.Exists(attack => attack.Attacker == attacker);

    /// <summary>The engine's own attack for <paramref name="attacker"/>, or null where nothing requires it to attack.</summary>
    public AttackMessage? SuggestedFor(ViewHandle attacker) => Suggested.Find(attack => attack.Attacker == attacker);

    /// <summary>
    /// The attacks of <paramref name="attacker"/> the host's verdict on no attacks offers (<see cref="Verdict"/>): those some
    /// legal declaration has - for a creature an effect requires to attack a certain player, only those obeying as much.
    /// </summary>
    public List<AttackMessage> OfferedFor(ViewHandle attacker) => Verdict.Addable.FindAll(attack => attack.Attacker == attacker);
}

/// <summary>
/// What one attack costs to declare (CR 508.1d, 508.1h): <see cref="Attacker"/> attacking the seat
/// <see cref="TargetPlayer"/>, or <see cref="TargetPermanent"/> when that is not none.
/// </summary>
[MessagePackObject]
public sealed class AttackCostMessage
{
    [Key(0)]
    public ViewHandle Attacker { get; set; }

    [Key(1)]
    public int TargetPlayer { get; set; }

    [Key(2)]
    public ViewHandle TargetPermanent { get; set; }

    /// <summary>The cost in oracle notation, "{2}".</summary>
    [Key(3)]
    public string Cost { get; set; } = string.Empty;

    /// <summary>Its mana value, for adding costs up.</summary>
    [Key(4)]
    public int ManaValue { get; set; }

    /// <summary>Its generic part.</summary>
    [Key(5)]
    public int Generic { get; set; }
}

/// <summary>
/// One requirement on a declaration of attackers (CR 508.1d), one for each ability, effect and player it comes from, for
/// showing the player: <see cref="Attacker"/> attacks, whatever it attacks; or attacks the seat <see cref="TargetPlayer"/>
/// or the permanent <see cref="TargetPermanent"/> ("attacks that player this combat if able"); or attacks a player other
/// than the seat <see cref="OtherThan"/> (a goad's, CR 701.15b). How many must be obeyed, and whether a declaration obeys
/// them, is the host's to judge (<see cref="DeclareAttackersPromptMessage.Verdict"/>, <see cref="CheckAttacksMessage"/>).
/// </summary>
[MessagePackObject]
public sealed class AttackRequirementMessage
{
    [Key(0)]
    public ViewHandle Attacker { get; set; }

    /// <summary>The seat it must attack, or 0.</summary>
    [Key(1)]
    public int TargetPlayer { get; set; }

    /// <summary>The planeswalker or battle it must attack, or none.</summary>
    [Key(2)]
    public ViewHandle TargetPermanent { get; set; }

    /// <summary>A seat it must attack a player other than, or 0.</summary>
    [Key(3)]
    public int OtherThan { get; set; }
}

/// <summary>
/// What the rules make of a declaration of attackers (CR 508.1a-d), as the host's one validator judges it: whether it may be
/// declared as it is, why not, what it costs, and the attacks that some legal declaration extending it has.
/// </summary>
[MessagePackObject]
public sealed class AttackVerdictView
{
    /// <summary>The declaration may be made as it is.</summary>
    [Key(0)]
    public bool Complete { get; set; }

    /// <summary>The attacks, by creatures not attacking, that some legal declaration extending it has.</summary>
    [Key(1)]
    public List<AttackMessage> Addable { get; set; } = [];

    /// <summary>Why it may not be declared as it is; null when it may.</summary>
    [Key(2)]
    public AttackIssueView? Issue { get; set; }

    /// <summary>What declaring it costs (CR 508.1h), in mana symbols; empty when nothing.</summary>
    [Key(3)]
    public string Cost { get; set; } = string.Empty;

    /// <summary>
    /// Nothing is required of the declaration: any declaration of <see cref="Addable"/> attacks, each creature attacking once,
    /// is legal, so a client need not ask the host about each one (the prompt's own verdict says so for the whole question).
    /// False wherever a target caps how many creatures may attack it (CR 508.1c), as two attacks addable on their own may be
    /// one too many together.
    /// </summary>
    [Key(4)]
    public bool Free { get; set; }
}

/// <summary>
/// Why a declaration of attackers may not be made as it is, to caption as <see cref="Creature"/>, <see cref="Words"/>, then
/// the seat <see cref="OtherPlayer"/> or the permanent <see cref="OtherPermanent"/> where there is one ("Ursine Monstrosity
/// must attack Bot2"), with the Comprehensive Rules number of the rule it breaks ("508.1d").
/// </summary>
[MessagePackObject]
public sealed class AttackIssueView
{
    [Key(0)]
    public ViewHandle Creature { get; set; }

    [Key(1)]
    public string Words { get; set; } = string.Empty;

    [Key(2)]
    public int OtherPlayer { get; set; }

    [Key(3)]
    public ViewHandle OtherPermanent { get; set; }

    [Key(4)]
    public string Rule { get; set; } = string.Empty;
}

/// <summary>
/// "Judge this declaration of attackers for the pending question": answered with an <see cref="AttacksVerdictMessage"/>, to
/// this seat alone. Nothing in the game moves. For a <see cref="DeclareAttackersPromptMessage"/> only.
/// </summary>
[MessagePackObject]
public sealed class CheckAttacksMessage : ClientMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    [Key(1)]
    public List<AttackMessage> Attacks { get; set; } = [];

    public override string Describe() => $"check {RequestId}: {Attacks.Count} attacks";
}

/// <summary>The rules' verdict on a declaration of attackers a <see cref="CheckAttacksMessage"/> asked about.</summary>
[MessagePackObject]
public sealed class AttacksVerdictMessage : ServerMessage
{
    [Key(0)]
    public int RequestId { get; set; }

    /// <summary>The declaration the verdict is on, as it was asked.</summary>
    [Key(1)]
    public List<AttackMessage> Attacks { get; set; } = [];

    [Key(2)]
    public AttackVerdictView Verdict { get; set; } = new();

    public override string Describe() => $"attacks verdict {RequestId}: {(Verdict.Complete ? "complete" : "incomplete")}, {Verdict.Addable.Count} addable";
}

/// <summary>One defending player declares blockers (CR 509.1).</summary>
[MessagePackObject]
public sealed class DeclareBlockersPromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    [Key(1)]
    public List<ViewHandle> Eligible { get; set; } = [];

    /// <summary>Attackers attacking this player: the only ones their creatures may block.</summary>
    [Key(2)]
    public List<ViewHandle> Attackers { get; set; } = [];

    /// <summary>
    /// Every blocker and attacker that may be paired. Narrower than <see cref="Eligible"/> against
    /// <see cref="Attackers"/> once a restriction such as flying (CR 702.9b) applies; a block outside this list is refused.
    /// </summary>
    [Key(3)]
    public List<BlockMessage> Allowed { get; set; } = [];

    /// <summary>
    /// The fewest creatures that may block each of <see cref="Attackers"/>, index for index: 2 for menace, 3 for "except
    /// by three or more", otherwise 1. A declaration giving an attacker some blockers but fewer is refused.
    /// </summary>
    [Key(4)]
    public List<int> MinimumBlockers { get; set; } = [];

    /// <summary>
    /// The most creatures that may block each of <see cref="Attackers"/>, index for index: 1 for "can't be blocked by more
    /// than one creature" (CR 509.1b), 0 for any number. A declaration giving an attacker more is refused.
    /// </summary>
    [Key(5)]
    public List<int> MaximumBlockers { get; set; } = [];

    /// <summary>
    /// The requirements effects put on the declaration (CR 509.1c), one each, for the player to see: Lure's "all creatures
    /// able to block enchanted creature do so" makes one of each creature able to block the creature it enchants; "target
    /// creature blocks this turn if able" one of that creature with <see cref="BlockMessage.Attacker"/>
    /// <see cref="ViewHandle.None"/>, obeyed by blocking any attacker; "must be blocked if able" one of that attacker with
    /// <see cref="BlockMessage.Blocker"/> <see cref="ViewHandle.None"/>, obeyed by any creature blocking it. Empty where nothing
    /// is required. Whether a declaration obeys enough of them is the host's to judge (<see cref="Verdict"/>,
    /// <see cref="CheckBlocksMessage"/>).
    /// </summary>
    [Key(6)]
    public List<BlockMessage> Requirements { get; set; } = [];

    /// <summary>
    /// How many of <see cref="Requirements"/> the declaration must obey: the most any declaration that obeys every
    /// restriction does (CR 509.1c). A declaration that obeys fewer is refused.
    /// </summary>
    [Key(7)]
    public int MostRequirements { get; set; }

    /// <summary>A declaration that obeys <see cref="MostRequirements"/> of them, the engine's own; empty where nothing is required.</summary>
    [Key(8)]
    public List<BlockMessage> Suggested { get; set; } = [];

    /// <summary>
    /// The attacking band each of <see cref="Attackers"/> is in, index for index, 0 for none (CR 702.22h): a creature blocking
    /// one member of a band blocks every member, so it obeys a requirement to block any of them.
    /// </summary>
    [Key(9)]
    public List<int> Bands { get; set; } = [];

    /// <summary>
    /// The rules' verdict on declaring no blockers (CR 509.1a-c): whether that may be declared, why not, and the blocks that
    /// may be declared first. Every other declaration is judged by the host as it is made (<see cref="CheckBlocksMessage"/>).
    /// </summary>
    [Key(10)]
    public BlockVerdictView Verdict { get; set; } = new() { Complete = true };

    /// <summary>Whether a creature blocking <paramref name="declared"/> blocks <paramref name="attacker"/>: the same creature, or a member of its band.</summary>
    public bool BlocksThrough(ViewHandle declared, ViewHandle attacker)
    {
        if (declared == attacker)
        {
            return true;
        }

        int Band(ViewHandle a)
        {
            int at = Attackers.IndexOf(a);
            return at >= 0 && at < Bands.Count ? Bands[at] : 0;
        }

        return Band(declared) != 0 && Band(declared) == Band(attacker);
    }

    /// <summary>The fewest creatures that may block the attacker at <paramref name="index"/>, at least 1.</summary>
    public int MinimumFor(int index) => index >= 0 && index < MinimumBlockers.Count ? Math.Max(MinimumBlockers[index], 1) : 1;

    /// <summary>The most creatures that may block the attacker at <paramref name="index"/>; <see cref="int.MaxValue"/> for any number.</summary>
    public int MaximumFor(int index) =>
        index >= 0 && index < MaximumBlockers.Count && MaximumBlockers[index] > 0 ? MaximumBlockers[index] : int.MaxValue;

    /// <summary>Blocking with nobody, or where an effect requires blocks (CR 509.1c) the engine's own declaration, which obeys them.</summary>
    public override AnswerPayload Default() => new BlockersDeclaredAnswer { Blocks = [.. Suggested] };
}

/// <summary>
/// This client divides the combat damage of each of their creatures that has more than one place to send it
/// (CR 510.1c-d, 702.19b): an attacker among its blockers and, with trample, the player it is attacking; a blocker
/// among the attackers it blocks. Answered with a <see cref="CombatDamageAssignedAnswer"/>.
/// </summary>
[MessagePackObject]
public sealed class AssignCombatDamagePromptMessage : PromptPayload
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>One per creature with a division to make, in combat order.</summary>
    [Key(1)]
    public List<DamageDivisionMessage> Divisions { get; set; } = [];

    /// <summary>The engine's own division of each creature's damage, which is always legal.</summary>
    public override AnswerPayload Default()
    {
        var answer = new CombatDamageAssignedAnswer();

        foreach (DamageDivisionMessage division in Divisions)
        {
            for (int i = 0; i < division.Creatures.Count; i++)
            {
                int amount = i < division.Suggested.Count ? division.Suggested[i] : 0;
                if (amount > 0)
                {
                    answer.Shares.Add(new DamageShareMessage { Source = division.Source, Creature = division.Creatures[i], Amount = amount });
                }
            }

            if (division.SuggestedToDefender > 0)
            {
                answer.Shares.Add(new DamageShareMessage { Source = division.Source, Creature = ViewHandle.None, Amount = division.SuggestedToDefender });
            }
        }

        return answer;
    }
}

/// <summary>One creature's combat damage and where it may go (CR 510.1).</summary>
[MessagePackObject]
public sealed class DamageDivisionMessage
{
    /// <summary>The attacking or blocking creature whose damage this is.</summary>
    [Key(0)]
    public ViewHandle Source { get; set; }

    /// <summary>How much it assigns: its power.</summary>
    [Key(1)]
    public int Damage { get; set; }

    /// <summary>What it may assign damage to: an attacker's blockers, or a blocker's attackers, in declaration order.</summary>
    [Key(2)]
    public List<ViewHandle> Creatures { get; set; } = [];

    /// <summary>
    /// Lethal damage for each creature, index for index: what must reach every one of them before any damage may go to
    /// the player (CR 702.19b). 1 from a source with deathtouch.
    /// </summary>
    [Key(3)]
    public List<int> Lethal { get; set; } = [];

    /// <summary>Trample: damage beyond lethal to every creature may go to the player being attacked instead.</summary>
    [Key(4)]
    public bool MayAssignToDefender { get; set; }

    /// <summary>The player the attacker is attacking, or 0 for a blocker or for an attacker attacking a planeswalker.</summary>
    [Key(5)]
    public int Defender { get; set; }

    /// <summary>The engine's own division, index for index with <see cref="Creatures"/>.</summary>
    [Key(6)]
    public List<int> Suggested { get; set; } = [];

    /// <summary>The rest of the engine's own division: what it would send to the player or planeswalker.</summary>
    [Key(7)]
    public int SuggestedToDefender { get; set; }

    /// <summary>The planeswalker the attacker is attacking, when the excess may go there instead (CR 702.19c).</summary>
    [Key(8)]
    public ViewHandle DefenderPermanent { get; set; }

    /// <summary>
    /// The creature is another player's (CR 702.22j-k): banding gives this player the division of its combat damage among
    /// their own creatures it is blocked by or is blocking, as they choose - none of it to a player.
    /// </summary>
    [Key(9)]
    public bool Banding { get; set; }
}

/// <summary>Where one creature's combat damage goes, one share per recipient.</summary>
[MessagePackObject]
public sealed class DamageShareMessage
{
    [Key(0)]
    public ViewHandle Source { get; set; }

    /// <summary>The creature this share goes to, or no handle for the player the source is attacking.</summary>
    [Key(1)]
    public ViewHandle Creature { get; set; }

    [Key(2)]
    public int Amount { get; set; }
}

// --- Answers ---

[Union(0, typeof(PassAnswer))]
[Union(1, typeof(CardsChosenAnswer))]
[Union(2, typeof(TakeActionAnswer))]
[Union(3, typeof(AttackersDeclaredAnswer))]
[Union(4, typeof(BlockersDeclaredAnswer))]
[Union(5, typeof(CombatDamageAssignedAnswer))]
[Union(6, typeof(CardsDistributedAnswer))]
[Union(7, typeof(TargetsChosenAnswer))]
[Union(8, typeof(MayChosenAnswer))]
[Union(9, typeof(ModesChosenAnswer))]
[Union(10, typeof(ReplacementChosenAnswer))]
[Union(11, typeof(TriggersOrderedAnswer))]
[Union(12, typeof(CancelAnnouncementAnswer))]
[Union(13, typeof(CardNamedAnswer))]
[Union(14, typeof(PlayerChosenAnswer))]
[Union(15, typeof(ColorsChosenAnswer))]
[Union(16, typeof(TargetsDividedAnswer))]
[Union(17, typeof(CountersRemovedAnswer))]
[Union(18, typeof(AnnounceAnswer))]
public abstract class AnswerPayload;

/// <summary>The counters taken for a <see cref="RemoveCountersPromptMessage"/>: how many of each kind from each permanent.</summary>
[MessagePackObject]
public sealed class CountersRemovedAnswer : AnswerPayload
{
    [Key(0)]
    public List<CounterPoolMessage> Removals { get; set; } = [];
}

/// <summary>Gives up the spell or ability being announced, in answer to any prompt that <see cref="PromptMessage.MayCancel"/>.</summary>
[MessagePackObject]
public sealed class CancelAnnouncementAnswer : AnswerPayload;

/// <summary>
/// The value of X, or how many times the spell is kicked, announced at once (CR 601.2b): an answer to a
/// <see cref="PayCostsPromptMessage"/> that <see cref="PayCostsPromptMessage.AnnouncesXOrKicks"/>, from its
/// <see cref="PayCostsPromptMessage.LeastAnnounced"/> to its <see cref="PayCostsPromptMessage.MostAnnounced"/>, in place of stepping to it with IncreaseX and DecreaseX.
/// </summary>
[MessagePackObject]
public sealed class AnnounceAnswer : AnswerPayload
{
    [Key(0)]
    public int Value { get; set; }
}

/// <summary>The modes chosen for a <see cref="ChooseModesPromptMessage"/>, as indexes into its modes.</summary>
[MessagePackObject]
public sealed class ModesChosenAnswer : AnswerPayload
{
    [Key(0)]
    public List<int> Modes { get; set; } = [];
}

/// <summary>Whether this client does what a <see cref="MayPromptMessage"/> offered.</summary>
[MessagePackObject]
public sealed class MayChosenAnswer : AnswerPayload
{
    [Key(0)]
    public bool Accept { get; set; }
}

/// <summary>The seat this client chooses for a <see cref="ChoosePlayerPromptMessage"/>: one of its candidates.</summary>
[MessagePackObject]
public sealed class PlayerChosenAnswer : AnswerPayload
{
    [Key(0)]
    public int Player { get; set; }
}

/// <summary>The colors this client chooses for a <see cref="ChooseColorsPromptMessage"/>: as many as it asked for.</summary>
[MessagePackObject]
public sealed class ColorsChosenAnswer : AnswerPayload
{
    [Key(0)]
    public List<ColorView> Colors { get; set; } = [];
}

/// <summary>The card name this client chooses for a <see cref="NameCardPromptMessage"/>: one of its names.</summary>
[MessagePackObject]
public sealed class CardNamedAnswer : AnswerPayload
{
    [Key(0)]
    public string Name { get; set; } = string.Empty;
}

/// <summary>
/// Which of the replacement effects offered by a <see cref="ChooseReplacementPromptMessage"/> applies first, as an
/// index into its sources.
/// </summary>
[MessagePackObject]
public sealed class ReplacementChosenAnswer : AnswerPayload
{
    [Key(0)]
    public int Index { get; set; }
}

/// <summary>
/// The order the triggered abilities of an <see cref="OrderTriggersPromptMessage"/> go on the stack, as indexes into
/// its sources: every index exactly once, the last one named resolving first.
/// </summary>
[MessagePackObject]
public sealed class TriggersOrderedAnswer : AnswerPayload
{
    [Key(0)]
    public List<int> Order { get; set; } = [];
}

/// <summary>
/// Where this client puts each of the cards it looked at: <see cref="Placements"/> runs index for index with the
/// prompt's destinations, and together the lists name every card looked at exactly once.
/// <para>
/// Within a list the cards are in the order they are put there, which for the top of a library means the first card
/// listed becomes the top card, and for the bottom means the last listed becomes the bottom card. With
/// <see cref="Shuffle"/> set the lists are ignored and the library is shuffled.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class CardsDistributedAnswer : AnswerPayload
{
    [Key(0)]
    public List<List<ViewHandle>> Placements { get; set; } = [];

    [Key(1)]
    public bool Shuffle { get; set; }
}

[MessagePackObject]
public sealed class AttackMessage
{
    [Key(0)]
    public ViewHandle Attacker { get; set; }

    /// <summary>The seat attacked, when <see cref="TargetPermanent"/> is none.</summary>
    [Key(1)]
    public int TargetPlayer { get; set; }

    /// <summary>The planeswalker attacked instead of a seat (CR 508.1b), or <see cref="ViewHandle.None"/>.</summary>
    [Key(2)]
    public ViewHandle TargetPermanent { get; set; }
}

[MessagePackObject]
public sealed class AttackersDeclaredAnswer : AnswerPayload
{
    [Key(0)]
    public List<AttackMessage> Attacks { get; set; } = [];
}

[MessagePackObject]
public sealed class BlockMessage
{
    [Key(0)]
    public ViewHandle Blocker { get; set; }

    [Key(1)]
    public ViewHandle Attacker { get; set; }
}

[MessagePackObject]
public sealed class BlockersDeclaredAnswer : AnswerPayload
{
    [Key(0)]
    public List<BlockMessage> Blocks { get; set; } = [];
}

/// <summary>
/// How each creature in an <see cref="AssignCombatDamagePromptMessage"/> divides its damage: one share per recipient,
/// adding up to the creature's damage. A share with no creature handle goes to the player the source is attacking.
/// </summary>
[MessagePackObject]
public sealed class CombatDamageAssignedAnswer : AnswerPayload
{
    [Key(0)]
    public List<DamageShareMessage> Shares { get; set; } = [];
}

[MessagePackObject]
public sealed class PassAnswer : AnswerPayload;

/// <summary>The target chosen for a triggered ability: one of the candidates, or none where that is allowed.</summary>
[MessagePackObject]
public sealed class TargetsChosenAnswer : AnswerPayload
{
    [Key(0)]
    public List<TargetMessage> Targets { get; set; } = [];
}

/// <summary>The targets and division chosen for a <see cref="ChooseDividedTargetsPromptMessage"/> (CR 601.2d).</summary>
[MessagePackObject]
public sealed class TargetsDividedAnswer : AnswerPayload
{
    [Key(0)]
    public List<TargetShareMessage> Shares { get; set; } = [];
}

[MessagePackObject]
public sealed class CardsChosenAnswer : AnswerPayload
{
    [Key(0)]
    public List<ViewHandle> Cards { get; set; } = [];
}

[MessagePackObject]
public sealed class TakeActionAnswer : AnswerPayload
{
    [Key(0)]
    public ActionKindView Kind { get; set; }

    [Key(1)]
    public ViewHandle Source { get; set; }

    [Key(2)]
    public int AbilityIndex { get; set; }

    /// <summary>The option taken (its OptionId); zero names it by kind, source and ability index, refused where two options share them.</summary>
    [Key(15)]
    public int OptionId { get; set; }

    /// <summary>Targets chosen as the spell is announced (CR 601.2c).</summary>
    [Key(3)]
    public List<TargetMessage> Targets { get; set; } = [];

    /// <summary>The color, for a mana ability that adds mana of any one color.</summary>
    [Key(5)]
    public ColorView? Color { get; set; }

    /// <summary>
    /// The colors, one per mana and repeats allowed, for a mana ability that adds mana "in any combination of" some
    /// colors (CR 106.1b). Empty for every other action.
    /// </summary>
    [Key(10)]
    public List<ColorView> Colors { get; set; } = [];

    /// <summary>
    /// The permanent sacrificed to pay a mana ability's cost (CR 118.3, 701.21a). Null for every other action, and for
    /// a cost that sacrifices its own source, which needs no choice.
    /// </summary>
    [Key(11)]
    public ViewHandle? Sacrifice { get; set; }

    /// <summary>The controlled permanent tapped for the mana ability's chosen tap cost.</summary>
    [Key(12)]
    public ViewHandle? Tap { get; set; }

    /// <summary>The pool mana to put towards a spell, when the action assigns some. Null otherwise.</summary>
    [Key(6)]
    public ManaTypeView? Mana { get; set; }

    /// <summary>What that mana may be spent on, when the action assigns some.</summary>
    [Key(8)]
    public ManaSpendFilterView Filter { get; set; }

    /// <summary>What spending that mana does to whatever it pays for, alongside <see cref="Filter"/>.</summary>
    [Key(9)]
    public ManaSpendEffectView Effect { get; set; }

    /// <summary>The subtypes <see cref="Filter"/> names, as <see cref="RestrictedManaView.SpendSubtypes"/>.</summary>
    [Key(13)]
    public List<string> SpendSubtypes { get; set; } = [];

    /// <summary>The mana value <see cref="Filter"/> names, as <see cref="RestrictedManaView.SpendManaValue"/>.</summary>
    [Key(14)]
    public int SpendManaValue { get; set; }

    /// <summary>The answer that takes <paramref name="option"/> as offered, targets and the rest still to fill in.</summary>
    public static TakeActionAnswer Take(ActionOptionMessage option)
    {
        ArgumentNullException.ThrowIfNull(option);
        return new TakeActionAnswer
        {
            Kind = option.Kind,
            Source = option.Source,
            AbilityIndex = option.AbilityIndex,
            OptionId = option.OptionId,
            Tap = option.Tap,
            Filter = option.Filter,
            Effect = option.Effect,
            SpendSubtypes = [.. option.SpendSubtypes],
            SpendManaValue = option.SpendManaValue,
        };
    }
}

/// <summary>
/// How a client talks to a game, whether that game is across a network or running in the same process.
/// <para>
/// Polling rather than callbacks or async: the web client drains this on a timer, so the client needs no
/// threading and no synchronisation context of its own. The WebSocket implementation keeps a reader filling a
/// queue; the loopback implementation hands messages straight over. Either way the client code is identical,
/// which is what makes "host and play" the same path as joining a server rather than a second one.
/// </para>
/// </summary>
public interface IGameTransport : IDisposable
{
    bool IsConnected { get; }

    void Send(ClientMessage message);

    /// <summary>Takes everything received since the last call. Empty when nothing has arrived.</summary>
    IReadOnlyList<ServerMessage> Receive();
}

/// <summary>
/// This client is searching its own library (CR 701.19).
/// <para>
/// The only message that carries a library's contents, and only ever to its owner while the search is outstanding.
/// Afterwards the server drops their handles, so the order after the shuffle cannot be matched against what was seen.
/// </para>
/// </summary>
[MessagePackObject]
public sealed class SearchLibraryPromptMessage : PromptPayload, ISelectionPrompt
{
    [Key(0)]
    public int Player { get; set; }

    /// <summary>How many the search takes, as <see cref="CountKind"/> counts it, for wording it.</summary>
    [Key(1)]
    public int Count { get; set; }

    /// <summary>The whole library, which the searcher sees, in the order the search lists it.</summary>
    [Key(3)]
    public List<ViewHandle> Library { get; set; } = [];

    /// <summary>Where the chosen cards will go.</summary>
    [Key(5)]
    public SearchDestinationView Destination { get; set; }

    /// <summary>Whose library is searched: the searcher's own, or another player's (Deadly Cover-Up).</summary>
    [Key(6)]
    public int Owner { get; set; }

    /// <summary>
    /// How the search counts its cards: up to <see cref="Count"/> for a search with a stated quality, which may find nothing
    /// (CR 701.23b); as many as there are, up to it, for "a card"; any number.
    /// </summary>
    [Key(8)]
    public CountKindView CountKind { get; set; }

    /// <summary>What the search's rule calls the cards it looks for ("basic land"), empty for "card".</summary>
    [Key(9)]
    public string Kind { get; set; } = string.Empty;

    /// <summary>What the rule asks of the cards beyond their noun, in the words a caption puts after it ("with different names").</summary>
    [Key(10)]
    public List<string> Words { get; set; } = [];

    /// <summary>
    /// The rule's verdict on finding nothing yet: whether that is an answer (a search with a quality may always find
    /// nothing, CR 701.23b), and the cards that may be found first. Every selection after is checked with the host.
    /// </summary>
    [Key(11)]
    public SelectionVerdictView Verdict { get; set; } = new();

    [IgnoreMember]
    public IReadOnlyList<ViewHandle> Candidates => Library;

    /// <summary>For wording: fewer than <see cref="Count"/> may be found, none included (CR 701.23b), or any number.</summary>
    [IgnoreMember]
    public bool Optional => CountKind is CountKindView.UpTo or CountKindView.AnyNumber;

    /// <summary>For wording: "any number of" (CR 701.23), rather than "up to" so many.</summary>
    [IgnoreMember]
    public bool AnyNumber => CountKind == CountKindView.AnyNumber;

    /// <summary>Finding nothing where that is an answer (CR 701.23b); otherwise the first cards that may be found.</summary>
    public override AnswerPayload Default() => new CardsChosenAnswer { Cards = Verdict.Complete ? [] : [.. Verdict.Addable.Take(Count)] };
}
