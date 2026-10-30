using Cantrip.Core;
using Cantrip.Protocol;
using Cantrip.Server.Rules;

namespace Cantrip.Server;

/// <summary>
/// One game, with its clients.
/// <para>
/// The authority boundary. Clients send answers and receive views; everything else — what is legal, what the
/// rules do, what each player may know — happens on this side. A client could be hostile or simply stale and
/// the worst it can achieve is an <see cref="ErrorMessage"/>.
/// </para>
/// <para>
/// Transport-agnostic on purpose: it moves messages into and out of per-seat mailboxes and never touches a
/// socket. The same session therefore serves a dedicated server and a host-and-play client with no second code
/// path, which matters because a second path is a second set of bugs that only appear in one of the two modes.
/// </para>
/// <para>
/// Engine-agnostic too: the rules are an <see cref="IRulesHost"/>, which hands over views, events and prompts already
/// projected for each seat, so nothing here knows what a game state is. The native engine's sidecar is the host, and a
/// test may stand a double in its place.
/// </para>
/// </summary>
public sealed class GameSession : IDisposable
{
    private readonly IRulesHost _rules;
    private readonly List<PlayerId> _seats;
    private readonly Dictionary<PlayerId, Queue<ServerMessage>> _outbox = [];

    /// <summary>
    /// Requests published to clients, with the message each was sent as.
    /// <para>
    /// The message is kept, not just the seat, so a rejected answer can be met with the same prompt again. Without that
    /// the seat deadlocks: the server still holds the request, but the client cleared its copy when it answered, so
    /// neither side will ever move.
    /// </para>
    /// </summary>
    private readonly Dictionary<int, (PlayerId Seat, PromptMessage Message)> _published = [];

    /// <summary>How far through the rules' log every seat has been brought. One cursor, because all seats are
    /// published together.</summary>
    private long _sequence;
    private RulesStep? _latestStep;

    /// <summary>The game log for the whole table, kept for the whole game so any seat can be sent all of it again.</summary>
    private readonly PlayerLog _log = new();

    /// <summary>Each seat's deck as the game began, for its welcome: what it owned in the opening position.</summary>
    private readonly Dictionary<PlayerId, List<DeckCardView>> _decklists = [];

    /// <summary>
    /// The cards each seat has been sent as printed, in the order sent: its deck with its welcome, and every other card
    /// the first time it could see it. Sent again whole when the seat is brought back up to date.
    /// </summary>
    private readonly Dictionary<PlayerId, PrintedCards> _printed = [];

    /// <summary>
    /// A scenario's part of each step, for whatever drives the seats (docs/scenarios-in-the-browser.md); never sent to a
    /// client. Raised as the step is published, before its messages go out.
    /// </summary>
    public event Action<ScenarioProgress>? ScenarioProgressed;

    /// <summary>A session over whichever rules engine <paramref name="rules"/> is.</summary>
    public GameSession(IRulesHost rules)
    {
        ArgumentNullException.ThrowIfNull(rules);
        _rules = rules;
        _seats = [.. rules.Seats.Select(seat => new PlayerId(seat))];

        foreach (PlayerId id in _seats)
        {
            _outbox[id] = new Queue<ServerMessage>();
            _decklists[id] = [.. rules.Decklist(id.Value)];

            // A seat's own deck comes with its welcome, so it is never sent again as newly seen.
            _printed[id] = new PrintedCards(_decklists[id].Select(entry => entry.Card));
        }
    }

    /// <summary>
    /// The oracle of each card in a view that the seat is seeing for the first time this game, noted as sent
    /// (<see cref="PrintedCards.NewlySeen"/>).
    /// </summary>
    private List<ObjectView> NewlySeen(PlayerId seat, PlayerView view) => _printed[seat].NewlySeen(view, _rules.Oracle);

    private WelcomeMessage Welcome(PlayerId seat) => new()
    {
        Seat = seat.Value,
        PlayerNames = [.. _rules.SeatNames],
        Decklist = _decklists.TryGetValue(seat, out List<DeckCardView>? deck) ? deck : [],
    };

    /// <summary>The rules this session is played by.</summary>
    public IRulesHost Rules => _rules;

    public IReadOnlyList<PlayerId> Seats => _seats;

    public bool IsOver => _rules.IsOver || _forfeited;

    /// <summary>The turn the game is on, as the last position published says; zero before the first.</summary>
    public int Turn => AnyView?.Turn ?? 0;

    /// <summary>The step the game is in, as the last position published says.</summary>
    public TurnStepView Step => AnyView?.Step ?? TurnStepView.Untap;

    /// <summary>
    /// The seats that won, in seat order: the winning team's, once the game has been won by its rules, or none.
    /// </summary>
    public IReadOnlyList<int> WinningSeats => _rules.Outcome == GameOutcomeView.TeamWon
        ? [.. _rules.Seats.Where(seat => _rules.Team(seat) == _rules.WinningTeam).Order()]
        : [];

    /// <summary>Any seat's view of the last position published: what every seat sees of the turn and step.</summary>
    private PlayerView? AnyView => _latestStep?.Views.Values.FirstOrDefault();

    /// <summary>
    /// Ended from outside the rules - a match clock run out, a draw the players agreed to - rather than by the game itself.
    /// The engine's own draw of a loop of passes (<c>docs/rules-exceptions.md</c>, RX-4) is the rules', and not one of these.
    /// </summary>
    private bool _forfeited;

    /// <summary>How a game ended from outside its rules came out, for a seat brought back up to date after.</summary>
    private GameOverMessage? _forfeit;

    /// <summary>The seats the game is waiting on: every seat with a question published and not yet answered.</summary>
    public IReadOnlyCollection<int> WaitingOn => [.. _published.Values.Select(published => published.Seat.Value).Distinct()];

    /// <summary>
    /// Ends the game from outside the rules, as a match clock that has run out does: the seats named win - one seat's
    /// team, or nobody for a draw, which is also how a draw the players agree to ends the game (<see cref="DrawByAgreement"/>;
    /// <c>docs/rules-exceptions.md</c>, RX-4) - every seat is told the game is over, and nothing more is asked. The engine's
    /// own draw of a loop of passes (RX-4) ends the game through its rules instead.
    /// </summary>
    public void Forfeit(IReadOnlyList<int> winners)
    {
        ArgumentNullException.ThrowIfNull(winners);
        bool won = winners.Count > 0;
        int team = won ? _rules.Team(winners[0]) : 0;

        // The rules end their game too, so that what every end of a game reveals (CR 708.9: the face-down permanents and
        // spells) is told to every seat - its events, its views and its log lines - before the game is over.
        RulesStep? ended = _rules.IsOver ? null : _rules.End(won ? winners[0] : null);
        _forfeited = true;
        _published.Clear();
        _forfeit = new GameOverMessage { Outcome = won ? GameOutcomeView.TeamWon : GameOutcomeView.Draw, WinningTeam = team };
        if (ended is not null)
        {
            Publish(ended);
            return;
        }

        foreach (PlayerId id in Seats)
        {
            _outbox[id].Enqueue(new GameOverMessage { Outcome = _forfeit.Outcome, WinningTeam = _forfeit.WinningTeam });
        }
    }

    /// <summary>
    /// The seats still in the game: every seat that has not lost, conceded or left. Judged from every seat's view of the last
    /// position published - a seat any of them shows out is out, since a seat not answering may be shown a position from
    /// before the run being played (the host shows a run to its answering seat alone) - and from the seats this session
    /// has itself taken out (<see cref="Leave"/>).
    /// </summary>
    public IReadOnlyList<int> InGame =>
    [
        .. Seats.Select(seat => seat.Value)
            .Where(seat => !_left.Contains(seat))
            .Where(seat => _latestStep is not { } step || step.Views.Values.All(view =>
                view.Players.FirstOrDefault(player => player.Seat == seat) is not { } shown || shown.Status == PlayerStatusView.Active)),
    ];

    /// <summary>The seats this session has taken out of the game from outside it (<see cref="Leave"/>).</summary>
    private readonly HashSet<int> _left = [];

    /// <summary>
    /// Seats whose match clocks have run out at once (CR 104.3a, 800.4a): each loses and leaves, through the rules' own
    /// concession, and a game of more than two goes on for the rest. If every seat still in ran out together, nobody is left
    /// to win: the game ends as a draw from outside its rules (<see cref="Forfeit"/>).
    /// </summary>
    public void TimeOut(IReadOnlyCollection<int> seats)
    {
        ArgumentNullException.ThrowIfNull(seats);
        if (IsOver)
        {
            return;
        }

        List<int> standing = [.. InGame];
        if (standing.All(seats.Contains))
        {
            Forfeit([]);
            return;
        }

        // Every seat out of time leaves before a draw offered is judged, so what comes of it never turns on which seat's
        // number is lower: an offerer out of time withdraws its offer, and never draws the game by running out.
        _judgingHeld = true;
        try
        {
            foreach (int seat in seats.Order())
            {
                if (IsOver)
                {
                    return;
                }

                if (standing.Contains(seat))
                {
                    Leave(new PlayerId(seat));
                }
            }
        }
        finally
        {
            _judgingHeld = false;
        }

        JudgeOffer();
    }

    /// <summary>Whether a draw offered waits to be judged until the seats leaving together have all left (<see cref="TimeOut"/>).</summary>
    private bool _judgingHeld;

    /// <summary>A seat loses and leaves the game from outside it (<see cref="IRulesHost.Leave"/>); the rest play on.</summary>
    public void Leave(PlayerId seat)
    {
        _left.Add(seat.Value);
        RulesStep step = _rules.Leave(seat.Value);
        _published.Clear();
        Publish(step);
    }

    /// <summary>Whether a seat can concede now: only when that ends the game (<see cref="IRulesHost.CanConcede"/>).</summary>
    public bool CanConcede(PlayerId seat) => !_forfeited && _latestStep is not null && _rules.CanConcede(seat.Value);

    /// <summary>
    /// A seat concedes (CR 104.3a): the game ends with everyone else winning, and every seat is sent the last position,
    /// the log's word that it was conceded, and that the game is over. Whatever was being asked is asked no more.
    /// </summary>
    public void Concede(PlayerId seat)
    {
        if (!CanConcede(seat))
        {
            throw new InvalidOperationException($"Seat {seat.Value} cannot concede now.");
        }

        RulesStep step = _rules.Concede(seat.Value);
        _published.Clear();
        Publish(step);
    }

    /// <summary>The draw offered at the table, while one stands (docs/rules-exceptions.md, RX-4).</summary>
    private DrawOffer? _offer;

    private sealed class DrawOffer(int offerer)
    {
        public int Offerer { get; } = offerer;
        public SortedSet<int> Accepted { get; } = [];
        public SortedSet<int> Declined { get; } = [];
    }

    /// <summary>The seat whose draw offer stands, or zero for none. Withdrawn by any answer of its offerer's but a pass (<see cref="Passes"/>).</summary>
    public int DrawOfferedBy => _offer?.Offerer ?? 0;

    /// <summary>Whether the game ended as a draw its players agreed to (<see cref="AnswerDraw"/>).</summary>
    public bool DrawnByAgreement { get; private set; }

    /// <summary>
    /// The waits that run no clock for their first moments (<see cref="MatchClocks.Tick"/>): the priority the offering seat
    /// is asked for while its draw offer stands, by the request's id, so each of its later priorities is given the courtesy
    /// once. Empty with no offer standing, or while the offering seat is asked anything but priority.
    /// </summary>
    public IReadOnlyDictionary<int, int> Courtesies =>
        _offer is { } offer && PriorityAsked(offer.Offerer) is { } asked
            ? new Dictionary<int, int> { [offer.Offerer] = asked }
            : new Dictionary<int, int>();

    /// <summary>
    /// The request a seat is asked its own priority by, if that is what it is being asked: not the priority of a player it
    /// controls (CR 723.5b: a draw is not the controlled player's to offer, and the offer would pass their priority).
    /// </summary>
    private int? PriorityAsked(int seat) =>
        _published.Where(published => published.Value.Seat.Value == seat && published.Value.Message.ForSeat == 0
                && published.Value.Message.Prompt is PriorityPromptMessage)
            .Select(published => (int?)published.Key)
            .FirstOrDefault();

    /// <summary>
    /// Why a seat may not offer a draw now, or null when it may: only a seat still in the game, while it has priority, and
    /// while no offer stands - one made by another is accepted instead (<see cref="AnswerDraw"/>).
    /// </summary>
    public string? CannotOfferDraw(PlayerId seat)
    {
        if (IsOver || _latestStep is null)
        {
            return "the game is over.";
        }

        if (!InGame.Contains(seat.Value))
        {
            return "you are no longer in the game.";
        }

        if (_offer is { } offer)
        {
            return offer.Offerer == seat.Value ? "your offer of a draw already stands." : "a draw is already offered; accept it instead.";
        }

        return PriorityAsked(seat.Value) is null ? "a draw is offered only while you have priority." : null;
    }

    /// <summary>
    /// A seat offers a draw (docs/rules-exceptions.md, RX-4): every seat is told, each other player is to accept or decline,
    /// and the offer passes the seat's priority - an answer like any other, so a resumed game replays it.
    /// </summary>
    public void OfferDraw(PlayerId seat)
    {
        if (CannotOfferDraw(seat) is { } why)
        {
            throw new InvalidOperationException($"Seat {seat.Value} cannot offer a draw now: {why}");
        }

        int asked = PriorityAsked(seat.Value)!.Value;
        _offer = new DrawOffer(seat.Value);
        TellOffer();
        Answer(seat, new AnswerMessage { RequestId = asked, Answer = new PassAnswer() });
    }

    /// <summary>
    /// Why a seat may not answer the draw offered as it asks, or null when it may: a player other than the one who offered,
    /// still in the game, answers once when the offer is made; one that declined may accept later while it has priority.
    /// </summary>
    public string? CannotAnswerDraw(PlayerId seat, bool accept)
    {
        if (IsOver || _offer is not { } offer)
        {
            return "no draw is offered.";
        }

        if (offer.Offerer == seat.Value || !InGame.Contains(seat.Value))
        {
            return "you are not asked about this draw.";
        }

        if (offer.Accepted.Contains(seat.Value))
        {
            return "you have already accepted the draw.";
        }

        if (offer.Declined.Contains(seat.Value))
        {
            return !accept ? "you have already declined the draw."
                : PriorityAsked(seat.Value) is null ? "a draw you declined is accepted only while you have priority." : null;
        }

        return null;
    }

    /// <summary>
    /// A seat accepts or declines the draw offered. Once every other player still in the game has accepted, the game is a
    /// draw (<see cref="DrawByAgreement"/>).
    /// </summary>
    public void AnswerDraw(PlayerId seat, bool accept)
    {
        if (CannotAnswerDraw(seat, accept) is { } why)
        {
            throw new InvalidOperationException($"Seat {seat.Value} cannot answer the draw: {why}");
        }

        DrawOffer offer = _offer!;
        if (accept)
        {
            offer.Declined.Remove(seat.Value);
            offer.Accepted.Add(seat.Value);
        }
        else
        {
            offer.Declined.Add(seat.Value);
        }

        if (!JudgeOffer())
        {
            TellOffer();
        }
    }

    /// <summary>
    /// Judges the draw offered as the game stands, whenever a seat may have left it - an answer, a concession, a clock - or
    /// accepted: an offer whose offerer is out of the game is withdrawn, and one every player still in the game but its
    /// offerer has accepted ends the game a draw. Says whether it did.
    /// </summary>
    private bool JudgeOffer()
    {
        if (_offer is not { } offer || IsOver || _judgingHeld)
        {
            return false;
        }

        List<int> standing = [.. InGame];
        if (!standing.Contains(offer.Offerer))
        {
            WithdrawDraw();
            return false;
        }

        if (!standing.Where(seat => seat != offer.Offerer).All(offer.Accepted.Contains))
        {
            return false;
        }

        DrawByAgreement();
        return true;
    }

    /// <summary>
    /// Ends the game as a draw its players agreed to, from outside its rules as a match clock ends one (<see cref="Forfeit"/>):
    /// nobody wins, every seat is told, and nothing more is asked.
    /// </summary>
    private void DrawByAgreement()
    {
        _offer = null;
        DrawnByAgreement = true;
        Forfeit([]);
    }

    /// <summary>The draw offered is withdrawn, and every seat told so.</summary>
    private void WithdrawDraw()
    {
        _offer = null;
        TellOffer();
    }

    /// <summary>Tells every seat the draw offered as it stands.</summary>
    private void TellOffer()
    {
        foreach (PlayerId id in Seats)
        {
            _outbox[id].Enqueue(OfferMessage());
        }
    }

    private DrawOfferMessage OfferMessage() => _offer is { } offer
        ? new DrawOfferMessage { Offerer = offer.Offerer, Accepted = [.. offer.Accepted], Declined = [.. offer.Declined] }
        : new DrawOfferMessage();

    /// <summary>Seats everyone, starts the game, and publishes the opening position.</summary>
    public void Start()
    {
        foreach (PlayerId id in Seats)
        {
            _outbox[id].Enqueue(Welcome(id));
        }

        Publish(_rules.Start());
    }

    /// <summary>Takes everything queued for a seat.</summary>
    public IReadOnlyList<ServerMessage> Drain(PlayerId seat)
    {
        if (!_outbox.TryGetValue(seat, out Queue<ServerMessage>? queue) || queue.Count == 0)
        {
            return [];
        }

        List<ServerMessage> messages = [.. queue];
        queue.Clear();
        return messages;
    }

    /// <summary>Replaces an authenticated seat's undelivered mailbox with its current authorized view and choices.
    /// No rules step is taken and no historical animation is replayed.</summary>
    public void Resynchronize(PlayerId seat)
    {
        if (_latestStep is null || !_outbox.TryGetValue(seat, out Queue<ServerMessage>? queue))
        {
            throw new InvalidOperationException("Only a started session's member can be resynchronized.");
        }

        queue.Clear();
        queue.Enqueue(Welcome(seat));
        PlayerView view = _rules.View(seat.Value);
        ObjectFingerprint.Stamp(view);
        NewlySeen(seat, view);
        queue.Enqueue(new StateMessage
        {
            View = view,

            // Every card sent as printed so far, since a client brought back up to date may have lost them all.
            Printed = [.. _printed[seat].Cards],
        });
        queue.Enqueue(new EventsMessage { Sequence = _sequence });

        // The whole log, not just what is new: whatever this client had, and whatever it missed while it was away.
        queue.Enqueue(new LogMessage { Entries = [.. _log.For(seat.Value)], Complete = true });

        foreach ((PlayerId owner, PromptMessage prompt) in _published.Values)
        {
            if (owner == seat)
            {
                queue.Enqueue(prompt);
            }
        }

        if (_offer is not null && !IsOver)
        {
            queue.Enqueue(OfferMessage());
        }

        if (IsOver)
        {
            queue.Enqueue(_forfeit is { } forfeit
                ? new GameOverMessage { Outcome = forfeit.Outcome, WinningTeam = forfeit.WinningTeam }
                : new GameOverMessage { Outcome = _rules.Outcome, WinningTeam = _rules.WinningTeam });
        }
    }

    /// <summary>
    /// Handles one message from a client.
    /// <para>
    /// Every rejection path returns an error to that client and leaves the game untouched. A client cannot
    /// desynchronise the game by sending nonsense, only by being ignored.
    /// </para>
    /// </summary>
    public void Submit(PlayerId from, ClientMessage message)
    {
        ArgumentNullException.ThrowIfNull(message);

        switch (message)
        {
            case JoinMessage join:
                if (join.ProtocolVersion != PlayerView.ProtocolVersion)
                {
                    Reject(from, ErrorCode.ProtocolMismatch,
                        $"server speaks protocol {PlayerView.ProtocolVersion}, client speaks {join.ProtocolVersion}.");
                }

                break;

            case AnswerMessage answer:
                Answer(from, answer);
                break;

            case CheckSelectionMessage check:
                Check(from, check);
                break;

            case CheckBlocksMessage check:
                CheckBlocks(from, check);
                break;

            case CheckAttacksMessage check:
                CheckAttacks(from, check);
                break;

            case ConcedeMessage when CanConcede(from):
                Concede(from);
                break;

            case ConcedeMessage:
                Reject(from, ErrorCode.IllegalAnswer, "a concession is taken only when it ends the game.");
                break;

            case OfferDrawMessage when CannotOfferDraw(from) is { } why:
                Reject(from, ErrorCode.IllegalAnswer, why);

                // The client let go of its question as it offered, as an answer does: whatever it is asked is asked again.
                foreach ((PlayerId owner, PromptMessage prompt) in _published.Values.Where(published => published.Seat == from))
                {
                    Reprompt(owner, prompt);
                }

                break;

            case OfferDrawMessage:
                OfferDraw(from);
                break;

            case AnswerDrawMessage answer when CannotAnswerDraw(from, answer.Accept) is { } why:
                Reject(from, ErrorCode.IllegalAnswer, why);
                break;

            case AnswerDrawMessage answer:
                AnswerDraw(from, answer.Accept);
                break;

            default:
                Reject(from, ErrorCode.IllegalAnswer, $"unrecognised message {message.GetType().Name}.");
                break;
        }
    }

    private void Answer(PlayerId from, AnswerMessage message)
    {
        if (IsOver)
        {
            Reject(from, ErrorCode.StaleReply, "the game is over.");
            return;
        }

        if (!_published.TryGetValue(message.RequestId, out (PlayerId Seat, PromptMessage Message) outstanding))
        {
            // Routine after a reconnect: the client is answering something already resolved. Nothing to re-ask, because
            // whatever was being asked has moved on.
            Reject(from, ErrorCode.StaleReply, $"request {message.RequestId} is not outstanding.");
            return;
        }

        if (outstanding.Seat != from)
        {
            Reject(
                from,
                ErrorCode.NotYourTurnToAnswer,
                $"request {message.RequestId} belongs to seat {outstanding.Seat.Value}.");

            return;
        }

        RulesStep step;
        try
        {
            step = _rules.Answer(message.RequestId, from.Value, message.Answer);
        }
        catch (RulesRejectedException refused)
        {
            // The rules are the authority on legality, so their refusal is reported rather than pre-empted by a
            // second, possibly disagreeing, check out here. The question still stands, so it is asked again — unless
            // the reply was stale, when there is nothing left to ask.
            Reject(from, refused.Code, refused.Detail);
            if (refused.Code != ErrorCode.StaleReply)
            {
                Reprompt(from, outstanding.Message);
            }

            return;
        }

        _published.Remove(message.RequestId);

        // The offerer did anything other than pass: its draw offer is withdrawn (RX-4). Any answer counts, those the game
        // forces included - a target, a mode, attackers - but for one that declares nothing (no attackers, no blockers).
        bool withdrawn = _offer is { } offer && offer.Offerer == from.Value && !Passes(outstanding.Message.Prompt, message.Answer);
        if (withdrawn)
        {
            _offer = null;
        }

        Publish(step);
        if (withdrawn)
        {
            TellOffer();
        }
    }

    /// <summary>
    /// What the rules say of a partial selection for the seat's outstanding choice of cards, sent back to that seat alone as a
    /// <see cref="SelectionVerdictMessage"/>. Nothing in the game moves, and nothing is asked again: a check refused (a stale
    /// request, a card the seat cannot see) is only reported.
    /// </summary>
    private void Check(PlayerId from, CheckSelectionMessage message)
    {
        if (IsOver || !_published.TryGetValue(message.RequestId, out (PlayerId Seat, PromptMessage Message) outstanding))
        {
            Reject(from, ErrorCode.StaleReply, $"request {message.RequestId} is not outstanding.", message.RequestId);
            return;
        }

        if (outstanding.Seat != from)
        {
            Reject(from, ErrorCode.NotYourTurnToAnswer, $"request {message.RequestId} belongs to seat {outstanding.Seat.Value}.", message.RequestId);
            return;
        }

        SelectionVerdictView verdict;
        try
        {
            verdict = _rules.CheckSelection(message.RequestId, from.Value, message.Selected);
        }
        catch (RulesRejectedException refused)
        {
            Reject(from, refused.Code, refused.Detail, message.RequestId);
            return;
        }

        if (_outbox.TryGetValue(from, out Queue<ServerMessage>? queue))
        {
            queue.Enqueue(new SelectionVerdictMessage { RequestId = message.RequestId, Selected = message.Selected, Verdict = verdict });
        }
    }

    /// <summary>
    /// What the rules make of a declaration of attackers for the seat's outstanding declaration, sent back to that seat alone
    /// as an <see cref="AttacksVerdictMessage"/>. Nothing in the game moves; a check refused is only reported.
    /// </summary>
    private void CheckAttacks(PlayerId from, CheckAttacksMessage message)
    {
        if (IsOver || !_published.TryGetValue(message.RequestId, out (PlayerId Seat, PromptMessage Message) outstanding))
        {
            Reject(from, ErrorCode.StaleReply, $"request {message.RequestId} is not outstanding.", message.RequestId);
            return;
        }

        if (outstanding.Seat != from)
        {
            Reject(from, ErrorCode.NotYourTurnToAnswer, $"request {message.RequestId} belongs to seat {outstanding.Seat.Value}.", message.RequestId);
            return;
        }

        AttackVerdictView verdict;
        try
        {
            verdict = _rules.CheckAttacks(message.RequestId, from.Value, message.Attacks);
        }
        catch (RulesRejectedException refused)
        {
            Reject(from, refused.Code, refused.Detail, message.RequestId);
            return;
        }

        if (_outbox.TryGetValue(from, out Queue<ServerMessage>? queue))
        {
            queue.Enqueue(new AttacksVerdictMessage { RequestId = message.RequestId, Attacks = message.Attacks, Verdict = verdict });
        }
    }

    /// <summary>
    /// What the rules make of a declaration of blockers for the seat's outstanding declaration, sent back to that seat alone
    /// as a <see cref="BlocksVerdictMessage"/>. Nothing in the game moves; a check refused is only reported.
    /// </summary>
    private void CheckBlocks(PlayerId from, CheckBlocksMessage message)
    {
        if (IsOver || !_published.TryGetValue(message.RequestId, out (PlayerId Seat, PromptMessage Message) outstanding))
        {
            Reject(from, ErrorCode.StaleReply, $"request {message.RequestId} is not outstanding.", message.RequestId);
            return;
        }

        if (outstanding.Seat != from)
        {
            Reject(from, ErrorCode.NotYourTurnToAnswer, $"request {message.RequestId} belongs to seat {outstanding.Seat.Value}.", message.RequestId);
            return;
        }

        BlockVerdictView verdict;
        try
        {
            verdict = _rules.CheckBlocks(message.RequestId, from.Value, message.Blocks);
        }
        catch (RulesRejectedException refused)
        {
            Reject(from, refused.Code, refused.Detail, message.RequestId);
            return;
        }

        if (_outbox.TryGetValue(from, out Queue<ServerMessage>? queue))
        {
            queue.Enqueue(new BlocksVerdictMessage { RequestId = message.RequestId, Blocks = message.Blocks, Verdict = verdict });
        }
    }

    /// <summary>
    /// Whether an answer is a pass, as a standing draw offer judges it (RX-4): passing priority, or declaring no attackers or
    /// no blockers. Anything else withdraws the offer - a target, a mode, an optional choice declined.
    /// </summary>
    private static bool Passes(PromptPayload asked, AnswerPayload answer) => (asked, answer) switch
    {
        (PriorityPromptMessage, PassAnswer) => true,
        (DeclareAttackersPromptMessage, AttackersDeclaredAnswer { Attacks.Count: 0 }) => true,
        (DeclareBlockersPromptMessage, BlockersDeclaredAnswer { Blocks.Count: 0 }) => true,
        _ => false,
    };

    /// <summary>
    /// Asks again after refusing an answer.
    /// <para>
    /// A refusal leaves the request outstanding on this side while the client has already discarded its copy, so
    /// without this the seat would simply stop answering and the game would stall — which is how a rejection shows up
    /// in practice: not as an error a player can act on, but as a game that quietly stops.
    /// </para>
    /// <para>
    /// The identical message goes back out. Nothing changed when the answer was refused, so every handle in it still
    /// resolves, and re-deriving it could only introduce a difference from what the client was first shown.
    /// </para>
    /// </summary>
    private void Reprompt(PlayerId seat, PromptMessage prompt)
    {
        if (_outbox.TryGetValue(seat, out Queue<ServerMessage>? queue))
        {
            queue.Enqueue(prompt);
        }
    }

    /// <summary>
    /// Publishes the position to every seat, then any prompt to the seat that owes it.
    /// <para>
    /// The view goes out first so a client always has the state its prompt refers to, and the handles in a
    /// prompt always resolve against a view it has already received.
    /// </para>
    /// </summary>
    private void Publish(RulesStep step)
    {
        _latestStep = step;
        if (step.Scenario is { } progress)
        {
            ScenarioProgressed?.Invoke(progress);
        }

        foreach (PlayerId id in Seats)
        {
            PlayerView view = step.Views[id.Value];

            // Each object stamped with a digest of itself, so a client can tell what is unchanged since the last view
            // without working it out for every object again.
            ObjectFingerprint.Stamp(view);
            _outbox[id].Enqueue(new StateMessage { View = view, Printed = NewlySeen(id, view) });

            IReadOnlyList<(long Sequence, GameEventView View)> events =
                step.Events.TryGetValue(id.Value, out IReadOnlyList<(long Sequence, GameEventView View)>? batch) ? batch : [];
            if (events.Count > 0)
            {
                _outbox[id].Enqueue(new EventsMessage
                {
                    Events = [.. events.Select(e => e.View)],
                    Sequences = [.. events.Select(e => e.Sequence)],
                    Sequence = step.Sequence,
                });
            }

            // The log's lines for these events, written now from the view and events this seat was just sent, so
            // they name exactly what the seat could see.
            PlayerLog.Update written = _log.Write(id.Value, view, events);
            if (!written.IsEmpty)
            {
                _outbox[id].Enqueue(new LogMessage { Entries = [.. written.Added], Removed = [.. written.Removed] });
            }
        }

        _sequence = step.Sequence;

        foreach (RulesRequest request in step.Requests)
        {
            // Only what has not already gone out. The rules report every outstanding request, not just new ones,
            // so when several are outstanding at once — each defending player declaring blocks, for instance — every
            // answer would otherwise re-send the prompts still waiting. The client would answer the duplicate and be
            // told its reply was stale, through no fault of its own.
            if (_published.ContainsKey(request.RequestId))
            {
                continue;
            }

            var seat = new PlayerId(request.Seat);
            var prompt = new PromptMessage
            {
                RequestId = request.RequestId,
                Prompt = request.Prompt,
                MayCancel = request.MayCancel,
                Presentation = request.Presentation,
                ForSeat = request.ForSeat == request.Seat ? 0 : request.ForSeat,
            };

            _published[request.RequestId] = (seat, prompt);
            _outbox[seat].Enqueue(prompt);
        }

        if (step.Finished)
        {
            // A game ended from outside its rules (Forfeit) is over as that says, whatever the rules made of it.
            foreach (PlayerId id in Seats)
            {
                _outbox[id].Enqueue(_forfeit is { } forfeit
                    ? new GameOverMessage { Outcome = forfeit.Outcome, WinningTeam = forfeit.WinningTeam }
                    : new GameOverMessage { Outcome = _rules.Outcome, WinningTeam = _rules.WinningTeam });
            }
        }

        // A seat the rules took out may have been the offerer, or the last one yet to accept.
        JudgeOffer();
    }

    private void Reject(PlayerId seat, ErrorCode code, string detail, int requestId = 0)
    {
        if (_outbox.TryGetValue(seat, out Queue<ServerMessage>? queue))
        {
            queue.Enqueue(new ErrorMessage { Code = code, Detail = detail, RequestId = requestId });
        }
    }

    /// <summary>Lets the rules go: the sidecar's game is closed.</summary>
    public void Dispose() => _rules.Dispose();
}
