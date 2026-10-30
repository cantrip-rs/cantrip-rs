using Cantrip.Protocol;

namespace Cantrip.Server.Rules;

/// <summary>
/// The whole of what a <see cref="GameSession"/> asks of a rules engine, in the wire's own vocabulary and nothing
/// engine-shaped.
/// <para>
/// A session's job is clients: mailboxes, who may answer what, asking again after a refusal, bringing a seat back up to
/// date. Everything about the rules — what is legal, what each seat may see, what happened — comes through here as
/// <c>Cantrip.Protocol</c> types, already projected for each seat and already in each seat's own handle space. That is
/// what lets the same session serve the native engine's sidecar (<see cref="NativeRulesHost"/>) and a test's double with
/// no second code path, and what makes this the shape of the sidecar's protocol: one advance in, one
/// <see cref="RulesStep"/> out.
/// </para>
/// </summary>
public interface IRulesHost : IDisposable
{
    /// <summary>The seats, numbered from one, in turn order.</summary>
    IReadOnlyList<int> Seats { get; }

    /// <summary>Each seat's name, index for index with <see cref="Seats"/>.</summary>
    IReadOnlyList<string> SeatNames { get; }

    /// <summary>The team a seat plays for (CR 810), for ending a game from outside its rules.</summary>
    int Team(int seat);

    /// <summary>
    /// A seat's deck as the game began, each card as printed, for its welcome: by name, with how many, in name order.
    /// Nothing in it names an object in the game.
    /// </summary>
    IReadOnlyList<DeckCardView> Decklist(int seat);

    /// <summary>Starts the game, replaying any history it was given, and says what the opening produced.</summary>
    RulesStep Start();

    /// <summary>
    /// Answers an outstanding request for a seat.
    /// </summary>
    /// <exception cref="RulesRejectedException">
    /// The answer was refused and the game is as it was: the request is still outstanding, except for a
    /// <see cref="ErrorCode.StaleReply"/>, which names one that is not.
    /// </exception>
    RulesStep Answer(int requestId, int seat, AnswerPayload answer);

    /// <summary>
    /// What the rule of an outstanding choice of cards says of the cards a seat has <paramref name="selected"/> so far
    /// (docs/host-protocol.md, "Checking a selection"): whether they are an answer, and which may be added. Nothing advances.
    /// </summary>
    /// <exception cref="RulesRejectedException">
    /// The request is not outstanding (<see cref="ErrorCode.StaleReply"/>), not the seat's, not a choice of cards, or a card
    /// is unknown to the seat.
    /// </exception>
    SelectionVerdictView CheckSelection(int requestId, int seat, IReadOnlyList<ViewHandle> selected) =>
        throw new RulesRejectedException(ErrorCode.IllegalAnswer, "this rules host judges no selections.");

    /// <summary>
    /// What the rules make of a declaration of blockers a seat is making for an outstanding declaration (docs/host-protocol.md,
    /// "Checking a declaration of blockers"): whether it may be declared, why not, and which blocks may be added. Nothing advances.
    /// </summary>
    /// <exception cref="RulesRejectedException">
    /// The request is not outstanding (<see cref="ErrorCode.StaleReply"/>), not the seat's, not a declaration of blockers, or a
    /// creature is unknown to the seat.
    /// </exception>
    BlockVerdictView CheckBlocks(int requestId, int seat, IReadOnlyList<BlockMessage> blocks) =>
        throw new RulesRejectedException(ErrorCode.IllegalAnswer, "this rules host judges no declarations of blockers.");

    /// <summary>
    /// What the rules make of a declaration of attackers a seat is making for an outstanding declaration (docs/host-protocol.md,
    /// "Checking a declaration of attackers"): whether it may be declared, why not, and which attacks may be added. Nothing advances.
    /// </summary>
    AttackVerdictView CheckAttacks(int requestId, int seat, IReadOnlyList<AttackMessage> attacks) =>
        throw new RulesRejectedException(ErrorCode.IllegalAnswer, "this rules host judges no declarations of attackers.");

    /// <summary>Whether a seat may concede now: only when that ends the game (CR 104.3a, 800.4a).</summary>
    bool CanConcede(int seat);

    /// <summary>The seat concedes; the game ends with everyone else winning.</summary>
    RulesStep Concede(int seat);

    /// <summary>
    /// The seat loses and leaves the game from outside it - its match clock has run out - as a concession does (CR 104.3a),
    /// whether or not that ends the game: in a game of more than two, the others play on without it (CR 800.4a).
    /// </summary>
    RulesStep Leave(int seat);

    /// <summary>
    /// Ends the game from outside its rules - a draw its players agreed to, a match clock run out, a table abandoned: won by
    /// <paramref name="winner"/>'s seat, or a draw for none - and says what its end produced: every face-down permanent and
    /// face-down spell revealed (CR 708.9). Null for a rules host that keeps no game to end.
    /// </summary>
    RulesStep? End(int? winner) => null;

    /// <summary>A seat's current view, for a client being brought back up to date. Nothing advances.</summary>
    PlayerView View(int seat);

    /// <summary>A card as printed, by the id a view names it by, or null for a card the rules do not know.</summary>
    ObjectView? Oracle(string cardId);

    bool IsOver { get; }

    GameOutcomeView Outcome { get; }

    /// <summary>The team that won, or zero while the game goes on or ended in a draw.</summary>
    int WinningTeam { get; }

    /// <summary>
    /// Every answer given through <see cref="Answer"/> since the game started, in order: what a game is resumed from,
    /// by starting again from the same deal and giving them again.
    /// </summary>
    IReadOnlyList<RulesAnswer> History { get; }
}

/// <summary>What one advance of the game produced, projected for every seat.</summary>
/// <param name="Requests">
/// Every question outstanding, not only the new ones: when several are outstanding at once — each defending player
/// declaring blocks — every answer reports the ones still waiting too, and the session sends only what it has not sent.
/// </param>
/// <param name="Views">Each seat's view of the position now, by seat.</param>
/// <param name="Events">
/// What happened since the last step as each seat may know it, by seat, each event with the sequence number of the
/// entry it came from, so a later retraction can name it and the same event reads as one log line for every seat.
/// </param>
/// <param name="Sequence">How far through the log every seat has now been brought.</param>
/// <param name="Finished">Whether the game is over.</param>
/// <param name="Public">
/// The position as a viewer who is no player sees it, for in-person play's tablets (docs/in-person.md); null unless the
/// game was dealt with a public view (<see cref="NativeRulesHostFactory.PublicView"/>). A session ignores it.
/// </param>
public sealed record RulesStep(
    IReadOnlyList<RulesRequest> Requests,
    IReadOnlyDictionary<int, PlayerView> Views,
    IReadOnlyDictionary<int, IReadOnlyList<(long Sequence, GameEventView View)>> Events,
    long Sequence,
    bool Finished,
    ScenarioProgress? Scenario = null,
    PublicProjection? Public = null);

/// <summary>
/// One step of the game as the public sees it: what is on the table for anyone to look at, with no seat's hidden
/// information, in a handle space of its own (docs/in-person.md).
/// </summary>
/// <param name="View">The public view of the position now. It is no seat's, so its <see cref="PlayerView.Viewer"/> is zero.</param>
/// <param name="Events">What happened since the last step as the public may know it, each with its entry's sequence number.</param>
/// <param name="Handles">
/// By seat, the public handle of each object that seat also sees, mapped to that seat's handle for the same object: how a
/// tap on a public board becomes a seat's answer. An object the seat does not see, or no longer names by the same handle,
/// has no entry.
/// </param>
public sealed record PublicProjection(
    PlayerView View,
    IReadOnlyList<(long Sequence, GameEventView View)> Events,
    IReadOnlyDictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>> Handles);

/// <summary>One question outstanding, in the handle space of the seat it is for.</summary>
/// <param name="RequestId">The host's id for it, echoed back with the answer. Never reused within a game.</param>
/// <param name="Seat">The seat that answers it.</param>
/// <param name="MayCancel">Asked while the seat is still announcing something it may give up (<see cref="PromptMessage.MayCancel"/>).</param>
/// <param name="Presentation">Wording facts for the client, or null where there are none (<see cref="PromptMessage.Presentation"/>).</param>
/// <param name="ForSeat">
/// The seat whose decision it is, when another seat controls that player and answers for them (CR 723.5); zero, or
/// <paramref name="Seat"/> itself, for the seat's own (<see cref="PromptMessage.ForSeat"/>).
/// </param>
public sealed record RulesRequest(
    int RequestId,
    int Seat,
    PromptPayload Prompt,
    bool MayCancel,
    PromptPresentationContext? Presentation,
    int ForSeat = 0);

/// <summary>An answer a seat gave, as it gave it.</summary>
public sealed record RulesAnswer(int Seat, AnswerPayload Answer);

/// <summary>The rules refused an answer, with the code the client is to be told and why.</summary>
public sealed class RulesRejectedException : Exception
{
    public RulesRejectedException(ErrorCode code, string detail) : base(detail)
    {
        Code = code;
        Detail = detail;
    }

    public ErrorCode Code { get; }

    public string Detail { get; }
}
