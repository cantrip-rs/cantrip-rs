using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>
/// A client's whole understanding of a game: the last view it was sent, and whatever it has been asked.
/// <para>
/// Thin in the literal sense. It holds no rules knowledge, computes nothing about legality, and never predicts
/// what an action will do. Everything it displays came from the server, and everything it offers came from a
/// prompt. The one thing it is allowed to decide is which of the offered answers to send.
/// </para>
/// <para>
/// Free of any UI, so the client's behaviour can be tested without a browser or a window. The renderer reads these
/// properties; it does not interpret them.
/// </para>
/// </summary>
public sealed class GameClient
{
    private readonly IGameTransport _transport;
    private readonly List<ErrorMessage> _errors = [];

    public GameClient(IGameTransport transport)
    {
        ArgumentNullException.ThrowIfNull(transport);
        _transport = transport;
    }

    /// <summary>Which seat this client plays, or zero before the welcome arrives.</summary>
    public int Seat { get; private set; }

    public IReadOnlyList<string> PlayerNames { get; private set; } = [];

    /// <summary>This seat's own deck as the game began, each card with how many and as printed; empty until welcomed.</summary>
    public IReadOnlyList<DeckCardView> Decklist { get; private set; } = [];

    private readonly Dictionary<string, ObjectView> _printed = new(StringComparer.Ordinal);

    /// <summary>
    /// Every card this client has been sent as printed - its oracle - by card id: its own deck, and each card it has seen
    /// in the game. Kept for the game.
    /// </summary>
    public IReadOnlyDictionary<string, ObjectView> Printed => _printed;

    /// <summary>A card was sent as printed for the first time: one this client had not seen before.</summary>
    public event Action<ObjectView>? PrintedSeen;

    private void NotePrinted(ObjectView card)
    {
        if (card.CardId.Length > 0 && _printed.TryAdd(card.CardId, card))
        {
            PrintedSeen?.Invoke(card);
        }
    }

    /// <summary>Private proof for this seat, retained across transport loss. Never expose in the table UI or logs.</summary>
    public MembershipMessage? Membership { get; private set; }

    public TableMessage? Table { get; private set; }

    /// <summary>
    /// Why this seat was given up - the player left a forming table, or did not say ready in time - once it has been;
    /// the seat is gone, and is not to be taken back.
    /// </summary>
    public SeatReleasedMessage? Released { get; private set; }

    /// <summary>Where this seat has moved to - another table, and the ticket for it - once it has.</summary>
    public TableMoveMessage? Moved { get; private set; }

    /// <summary>The draft as this seat last saw it, while the table is drafting; null otherwise.</summary>
    public DraftMessage? Draft { get; private set; }

    /// <summary>Every seat's match clock as last sent, or null for a game with none.</summary>
    public ClockMessage? Clock { get; private set; }

    /// <summary>When <see cref="Clock"/> arrived, so a running clock can be counted down from it.</summary>
    public DateTime ClockReceived { get; private set; }

    /// <summary>How long a seat has left on its match clock now, counting a running one down; null with no clock.</summary>
    public (TimeSpan Left, bool Running)? ClockOf(int seat)
    {
        if (Clock?.Clocks.FirstOrDefault(clock => clock.Seat == seat) is not { } clock) return null;
        TimeSpan left = TimeSpan.FromMilliseconds(clock.LeftMs) - (clock.Running ? DateTime.UtcNow - ClockReceived : TimeSpan.Zero);
        return (left < TimeSpan.Zero ? TimeSpan.Zero : left, clock.Running);
    }

    /// <summary>An event's pairings and standings as this seat last saw them, while its table shows them; null otherwise.</summary>
    public PairingsMessage? Pairings { get; private set; }

    /// <summary>The deck this seat is building, as the server last had it, while decks are built; null otherwise.</summary>
    public DeckBuildingMessage? DeckBuilding { get; private set; }

    /// <summary>The most recent position, or null before the first one arrives.</summary>
    public PlayerView? View { get; private set; }

    /// <summary>What the server is waiting on from this client, or null when it is not this client's turn to answer.</summary>
    public PromptMessage? Prompt { get; private set; }

    public GameOverMessage? Outcome { get; private set; }

    /// <summary>
    /// What happened since the last pump, oldest first. Replaced each pump rather than accumulated: this is what a
    /// renderer animates, and holding a whole game's history would be a leak with no reader.
    /// </summary>
    public IReadOnlyList<GameEventView> RecentEvents { get; private set; } = [];

    private readonly Dictionary<GameEventView, long> _sequences = new(ReferenceEqualityComparer.Instance);

    /// <summary>
    /// The server's log sequence for one of <see cref="RecentEvents"/>, or -1 for an event not in the latest batch. What
    /// an <see cref="EventsRetractedEvent"/> names events by.
    /// </summary>
    public long SequenceOf(GameEventView entry) => _sequences.TryGetValue(entry, out long sequence) ? sequence : -1;

    /// <summary>How far through the server's log this client has been brought.</summary>
    public long EventSequence { get; private set; }

    public bool IsOver => Outcome is not null;

    /// <summary>
    /// Rejections from the server, newest last. Kept rather than thrown: a stale reply being refused is a
    /// normal consequence of reconnecting, not a fault.
    /// </summary>
    public IReadOnlyList<ErrorMessage> Errors => _errors;

    /// <summary>True when the server has asked something this client has not yet answered.</summary>
    public bool IsWaitingOnMe => Prompt is not null;

    /// <summary>
    /// The seat whose decision the outstanding prompt is: this seat, or a player it controls (CR 723.5,
    /// <see cref="PromptMessage.ForSeat"/>), whose resources pay for what it decides (CR 723.5a) - "you" in the prompt is that
    /// player. This seat with nothing asked.
    /// </summary>
    public int ActingSeat => Prompt is { ForSeat: > 0 } prompt ? prompt.ForSeat : Seat;

    /// <summary>
    /// The seat this one controls now (CR 723), whose hand and hidden cards it sees and whose questions it answers, or zero.
    /// </summary>
    public int ControlledSeat => View?.Players.FirstOrDefault(player => player.ControlledBy == Seat && player.Seat != Seat)?.Seat ?? 0;

    /// <summary>
    /// Applies everything the transport has received. Called once per frame by the renderer.
    /// </summary>
    /// <returns>How many messages were applied, so a caller can tell whether anything changed.</returns>
    public int Pump()
    {
        IReadOnlyList<ServerMessage> messages = _transport.Receive();
        int applied = PumpMessages(messages);
        if (_settling is not null)
        {
            Settle();
        }

        _blocks.Settle();
        _attacks.Settle();

        return applied;
    }

    private int PumpMessages(IReadOnlyList<ServerMessage> messages)
    {
        List<GameEventView>? events = null;
        _sequences.Clear();

        foreach (ServerMessage message in messages)
        {
            if (message is EventsMessage batch)
            {
                // Batches from one pump are concatenated, so a renderer that draws once per frame still sees
                // everything in order rather than only the last batch.
                events ??= [];
                events.AddRange(batch.Events);
                EventSequence = batch.Sequence;

                for (int i = 0; i < batch.Events.Count && i < batch.Sequences.Count; i++)
                {
                    _sequences[batch.Events[i]] = batch.Sequences[i];
                }

                continue;
            }

            Apply(message);
        }

        RecentEvents = events ?? [];
        return messages.Count;
    }

    /// <summary>Whether the transport still has a connection. Always true for a game hosted in this process.</summary>
    public bool IsConnected => _transport.IsConnected;

    /// <summary>Asks for a seat. A <paramref name="ticket"/> from the host claims the seat it was issued for.</summary>
    public void Join(string playerName, int preferredSeat = 0, string? ticket = null) =>
        _transport.Send(new JoinMessage { PlayerName = playerName, PreferredSeat = preferredSeat, Ticket = ticket ?? string.Empty });

    public void Resume(MembershipMessage membership)
    {
        ArgumentNullException.ThrowIfNull(membership);
        _transport.Send(new ResumeMessage { MembershipId = membership.MembershipId, ReconnectProof = membership.ReconnectProof });
    }

    public void ConfigurePlayer(string name, string deck) =>
        _transport.Send(new ConfigurePlayerMessage { Name = name, Deck = deck });

    public void ConfigureTable(IEnumerable<string> sets) =>
        _transport.Send(new ConfigureTableMessage { Sets = [.. sets] });

    public void SetReady(bool ready) => _transport.Send(new ReadyMessage { Ready = ready });

    /// <summary>Takes or passes the pile this seat is looking at in the draft shown.</summary>
    public void Pick(bool take)
    {
        if (Draft is not { } draft || draft.Active != Seat)
        {
            throw new InvalidOperationException("It is not this seat's turn to pick.");
        }

        _transport.Send(new DraftPickMessage { Decision = draft.Decision, Take = take });
    }

    /// <summary>
    /// Takes the card at <paramref name="place"/> in the pack in front of this seat, in a booster draft, and with it the
    /// cards at <paramref name="also"/>: each an additional card, paid for by a card of the pool going into the pack
    /// (<see cref="DraftMessage.AdditionalPicks"/>).
    /// </summary>
    public void PickCard(int place, IReadOnlyList<int>? also = null)
    {
        if (Draft is not { } draft || draft.Pile.Count == 0)
        {
            throw new InvalidOperationException("There is no pack in front of this seat.");
        }

        if (place < 0 || place >= draft.Pile.Count)
        {
            throw new ArgumentOutOfRangeException(nameof(place), "That card is not in the pack.");
        }

        also ??= [];
        if (also.Count > draft.AdditionalPicks.Count || also.Any(other => other < 0 || other >= draft.Pile.Count || other == place) || also.Distinct().Count() != also.Count)
        {
            throw new ArgumentOutOfRangeException(nameof(also), "Those cards cannot be taken with the pick.");
        }

        _transport.Send(new DraftPickMessage { Decision = draft.Decision, Take = true, Card = place, Also = [.. also] });
    }

    /// <summary>
    /// Takes a row (0 to 2, from the top) or a column (3 to 5, from the left) of the grid in a grid draft, on this seat's
    /// pick.
    /// </summary>
    public void PickLine(int line)
    {
        if (Draft is not { Format: "grid" } draft || draft.Active != Seat)
        {
            throw new InvalidOperationException("It is not this seat's pick from the grid.");
        }

        if (line is < 0 or > 5)
        {
            throw new ArgumentOutOfRangeException(nameof(line), "A line is a row or a column of the grid.");
        }

        _transport.Send(new DraftPickMessage { Decision = draft.Decision, Take = true, Card = line });
    }

    /// <summary>
    /// Tells the server which row or column of the grid this seat has chosen without taking it yet, so that a pick clock
    /// that runs out takes that line.
    /// </summary>
    public void ChooseLine(int line)
    {
        if (Draft is not { Format: "grid" } draft || draft.Active != Seat || line is < 0 or > 5)
        {
            return;
        }

        _transport.Send(new DraftPickMessage { Decision = draft.Decision, Take = false, Card = line });
    }

    /// <summary>
    /// Answers a booster pick that is due (<see cref="DraftMessage.PickDue"/>) with no card chosen, so the server takes one
    /// at random without waiting any longer.
    /// </summary>
    public void PickNothing()
    {
        if (Draft is not { } draft || draft.Pile.Count == 0)
        {
            throw new InvalidOperationException("There is no pack in front of this seat.");
        }

        _transport.Send(new DraftPickMessage { Decision = draft.Decision, Take = false });
    }

    /// <summary>
    /// Sends this seat's deck as it now stands - the places in its pool of the main deck's cards and the basic lands
    /// added, and its companion's place (-1 for none, CR 702.139a) - and with <paramref name="submit"/> says it is the deck to play.
    /// </summary>
    public void BuildDeck(IEnumerable<int> main, IReadOnlyDictionary<string, int> basics, bool submit, int companion = -1)
    {
        ArgumentNullException.ThrowIfNull(main);
        ArgumentNullException.ThrowIfNull(basics);
        if (DeckBuilding is not { } building)
        {
            throw new InvalidOperationException("No deck is being built.");
        }

        _transport.Send(new BuildDeckMessage
        {
            Change = ++DeckChangesSent,
            Main = [.. main],
            Basics = [.. basics.Select(land => new BasicLandCount { Name = land.Key, Count = land.Value })],
            Submit = submit,
            Companion = companion,
        });
    }

    /// <summary>
    /// How many deck changes this client has sent. A deck view whose <see cref="DeckBuildingMessage.Change"/> is below
    /// it is from before the latest of them, and shows a deck this client has already changed.
    /// </summary>
    public int DeckChangesSent { get; private set; }

    /// <summary>Whether every deck change this client has sent has come back in the deck view.</summary>
    public bool DeckCaughtUp => DeckBuilding is not { } building || building.Change >= DeckChangesSent;

    /// <summary>Explicit departure revokes membership; unlike Disconnect, it cannot be resumed.</summary>
    public void Leave() => _transport.Send(new LeaveMessage());

    /// <summary>Concedes the game being played: this seat loses it, and it is over.</summary>
    public void Concede() => _transport.Send(new ConcedeMessage());

    /// <summary>
    /// The draw offered at the table, as last sent (docs/rules-exceptions.md, RX-4), or null while none stands.
    /// </summary>
    public DrawOfferMessage? DrawOffer { get; private set; }

    /// <summary>Whether this seat is still in the game being played.</summary>
    private bool InGame => !IsOver && View?.Players.FirstOrDefault(player => player.Seat == Seat) is { Status: PlayerStatusView.Active };

    /// <summary>Whether this seat holds priority now: what it is asked is a priority prompt, its own or a player's it controls.</summary>
    public bool HasPriority => Prompt?.Prompt is PriorityPromptMessage;

    /// <summary>
    /// Whether this seat may offer a draw now: it is in the game, it has priority of its own - not a controlled player's,
    /// which the offer would pass (CR 723.5b) - and no offer stands.
    /// </summary>
    public bool CanOfferDraw => InGame && HasPriority && Prompt is { ForSeat: 0 } && DrawOffer is null;

    /// <summary>
    /// Whether another player's draw offer is waiting on this seat's first answer: it has neither accepted nor declined it.
    /// </summary>
    public bool DrawAskedOfMe => InGame && !_drawAnswerSent && DrawOffer is { } offer && offer.Offerer != Seat
        && !offer.Accepted.Contains(Seat) && !offer.Declined.Contains(Seat);

    /// <summary>
    /// This seat has answered the draw offered and the server's word on it - the offer with this seat among those who
    /// answered, or gone - has not come back yet, so it is not answered twice: another seat's answer may come back first.
    /// </summary>
    private bool _drawAnswerSent;

    /// <summary>The answer sent is an acceptance of an offer this seat had declined: landed once it is among those who accepted.</summary>
    private bool _acceptAfterDecline;

    /// <summary>
    /// Whether this seat may accept the draw offered now: when it is first asked, or, having declined, at any priority.
    /// </summary>
    public bool CanAcceptDraw => DrawAskedOfMe
        || (InGame && !_drawAnswerSent && DrawOffer is { } offer && offer.Offerer != Seat && offer.Declined.Contains(Seat) && HasPriority);

    /// <summary>
    /// Offers the other players a draw, passing this seat's priority: the offer stands until this seat answers anything but a
    /// pass (declaring no attackers or no blockers is one). The prompt it answers is cleared, as an answer clears it.
    /// </summary>
    public void OfferDraw()
    {
        if (Prompt is { } asked)
        {
            LastAnswered = asked.RequestId;
        }

        Prompt = null;
        _transport.Send(new OfferDrawMessage());
    }

    /// <summary>Accepts or declines the draw another player offered.</summary>
    public void AnswerDraw(bool accept)
    {
        _drawAnswerSent = true;
        _acceptAfterDecline = accept && DrawOffer?.Declined.Contains(Seat) == true;
        _transport.Send(new AnswerDrawMessage { Accept = accept });
    }

    /// <summary>Drops from the event this table is part of: this seat is paired no more.</summary>
    public void Drop() => _transport.Send(new DropMessage());

    /// <summary>
    /// Drops only the connection. Membership remains reserved for the server's reconnect grace period.
    /// </summary>
    public void Disconnect()
    {
        Prompt = null;
        _transport.Dispose();
    }

    /// <summary>The request this client last answered, 0 before its first answer.</summary>
    public int LastAnswered { get; private set; }

    /// <summary>
    /// Answers the outstanding prompt and clears it, so the same answer cannot be sent twice.
    /// <para>
    /// A choice of cards (<see cref="ISelectionPrompt"/>) is settled with the host first, whoever is answering: unless the
    /// host has already called the cards chosen an answer, they are checked (<see cref="Check"/>) and sent once it does. A
    /// selection the host refuses is mended one card at a time from what may be added - the cards proposed first, then the
    /// rest in the question's order - until it is one (<see cref="Settling"/>), so a player that picks without knowing the
    /// question's rule ("with different names", "total mana value 6 or less") still answers what the rules take. A choice of
    /// targets whose slot judges its targets together (<see cref="ChooseTargetPromptMessage.Verdict"/>: "that each have a
    /// different mana value X or less") is settled the same way, its object targets the cards and any player chosen kept as
    /// it is. The prompt is cleared as soon as this is called, as for any answer.
    /// </para>
    /// </summary>
    public void Answer(AnswerPayload answer)
    {
        ArgumentNullException.ThrowIfNull(answer);

        if (Prompt is null)
        {
            throw new InvalidOperationException("Nothing has been asked.");
        }

        // The cards the answer picks, for a question judged by a rule: a choice of cards' own, or the objects among the targets of
        // a slot that judges its targets together (the players chosen kept aside, as the rule does not judge them).
        (IReadOnlyList<ViewHandle> Cards, List<TargetMessage> Players)? picked = (Prompt.Prompt, answer) switch
        {
            (ISelectionPrompt { Judged: true } and not ChooseTargetPromptMessage, CardsChosenAnswer cards) => (cards.Cards, []),
            (ChooseTargetPromptMessage { Verdict: not null }, TargetsChosenAnswer targets) =>
                ([.. targets.Targets.Where(target => !target.IsPlayer).Select(target => target.Object)], [.. targets.Targets.Where(target => target.IsPlayer)]),
            _ => null,
        };
        if (picked is { } chosen && Verdict(chosen.Cards) is not { Complete: true })
        {
            // A question whose check was refused before is not settled the same way again, so an automatic seat cannot go
            // round forever: it is answered with the question's own default.
            if (_refusedSelections.Contains(Prompt.RequestId))
            {
                PromptMessage refused = Prompt;
                Prompt = null;
                Mended($"request {refused.RequestId} ({refused.Prompt.GetType().Name}): a check of it was refused before; sending the question's default for {Cards(chosen.Cards)}");
                LastAnswered = refused.RequestId;
                _transport.Send(new AnswerMessage { RequestId = refused.RequestId, Answer = refused.Prompt.Default() });
                return;
            }

            _settling = new Settling(Prompt, chosen.Cards) { Players = chosen.Players };
            Prompt = null;
            Settle();
            return;
        }

        if (Prompt.Prompt is DeclareBlockersPromptMessage && answer is BlockersDeclaredAnswer declared && _blocks.Takes(Prompt, declared.Blocks))
        {
            return;
        }

        if (Prompt.Prompt is DeclareAttackersPromptMessage && answer is AttackersDeclaredAnswer attacking && _attacks.Takes(Prompt, attacking.Attacks))
        {
            return;
        }

        var message = new AnswerMessage { RequestId = Prompt.RequestId, Answer = answer };

        // Cleared before sending. With a loopback transport the send runs the engine synchronously and can
        // publish the next prompt inside this call, which would otherwise be overwritten on the way out.
        Prompt = null;
        LastAnswered = message.RequestId;
        _transport.Send(message);
    }

    public void Pass() => Answer(new PassAnswer());

    // --- Choices of cards, judged by the host as they are made. ---

    /// <summary>The verdicts the host has given on selections of the request in <see cref="_verdictsFor"/>, by selection.</summary>
    private readonly Dictionary<string, SelectionVerdictView> _verdicts = new(StringComparer.Ordinal);

    /// <summary>The selections asked about and not yet answered.</summary>
    private readonly HashSet<string> _checking = new(StringComparer.Ordinal);

    private int _verdictsFor;

    /// <summary>Bumped whenever a verdict arrives, so a renderer knows to draw the choice again.</summary>
    public int VerdictRevision { get; private set; }

    private static string KeyOf(IEnumerable<ViewHandle> selected) =>
        string.Join(',', selected.Select(handle => handle.Value).Order());

    /// <summary>The request a choice of cards being made is for: the one asked, or the one being settled.</summary>
    private PromptMessage? Choosing => Prompt is { Prompt: ISelectionPrompt { Judged: true } } asked ? asked : _settling?.Asked;

    /// <summary>
    /// What the rules said of <paramref name="selected"/> for the choice of cards outstanding (<see cref="ISelectionPrompt"/>):
    /// whether the cards are an answer, and which may be added to them. For none chosen it is the prompt's own
    /// <see cref="ISelectionPrompt.Verdict"/>, and for the one card it calls an answer alone
    /// (<see cref="SelectionVerdictView.CompleteAlone"/>) a complete selection with nothing to add, known without asking; any other selection is known once the host has answered a <see cref="Check"/>
    /// of it, and null until then (or when nothing such is asked).
    /// </summary>
    public SelectionVerdictView? Verdict(IEnumerable<ViewHandle> selected)
    {
        ArgumentNullException.ThrowIfNull(selected);
        if (Choosing is not { Prompt: ISelectionPrompt selection } asked) return null;
        List<ViewHandle> cards = [.. selected];
        if (cards.Count == 0) return selection.Verdict;

        // A card the prompt calls an answer alone (a question of one card at most) is one, with nothing more to add: known
        // without a round trip to the host, which would say the same.
        if (cards.Count == 1 && selection.Verdict.CompleteAlone.Contains(cards[0]))
        {
            return new SelectionVerdictView { Complete = true };
        }

        return _verdictsFor == asked.RequestId && _verdicts.TryGetValue(KeyOf(cards), out SelectionVerdictView? verdict) ? verdict : null;
    }

    /// <summary>
    /// Asks the host what the outstanding choice of cards' rule says of <paramref name="selected"/> (docs/host-protocol.md,
    /// "Checking a selection"): its <see cref="SelectionVerdictMessage"/> arrives with a later <see cref="Pump"/>, and
    /// <see cref="Verdict"/> reads it. A selection already judged, or being judged, is not asked again.
    /// </summary>
    public void Check(IEnumerable<ViewHandle> selected)
    {
        ArgumentNullException.ThrowIfNull(selected);
        if (Choosing is not { } asked) return;
        List<ViewHandle> cards = [.. selected];
        if (Verdict(cards) is not null) return;
        if (_verdictsFor != asked.RequestId)
        {
            _verdicts.Clear();
            _checking.Clear();
            _verdictsFor = asked.RequestId;
        }

        if (!_checking.Add(KeyOf(cards))) return;
        _transport.Send(new CheckSelectionMessage { RequestId = asked.RequestId, Selected = cards });
    }

    private void Note(SelectionVerdictMessage message)
    {
        if (message.RequestId != _verdictsFor) return;
        string key = KeyOf(message.Selected);
        _checking.Remove(key);
        _verdicts[key] = message.Verdict;
        VerdictRevision++;
    }

    /// <summary>
    /// A proposal the rules refused was rebuilt (or sent as it was): the request, the proposal and what was sent, one line each.
    /// A mend is a client that proposed what the rules do not take - a bug in that client, kept visible rather than smoothed
    /// over. Also written to the error stream, and kept in <see cref="Mends"/>.
    /// </summary>
    public event Action<string>? MendedSelection;

    private readonly List<string> _mends = [];

    /// <summary>Every mend this client has made, oldest first.</summary>
    public IReadOnlyList<string> Mends => _mends;

    private void Mended(string line)
    {
        _mends.Add(line);
        Console.Error.WriteLine($"[client seat {Seat}] {line}");
        MendedSelection?.Invoke(line);
    }

    private string Cards(IEnumerable<ViewHandle> cards) =>
        "[" + string.Join(", ", cards.Select(card => Object(card)?.Name is { Length: > 0 } name ? $"{name} #{card.Value}" : $"#{card.Value}")) + "]";

    /// <summary>A choice of cards answered but not yet sent: being checked, and mended if the rules refuse it.</summary>
    private Settling? _settling;

    /// <summary>Whether an answer to a choice of cards or a declaration of blockers is being settled with the host before it is sent.</summary>
    public bool IsSettling => _settling is not null || _blocks.IsSettling || _attacks.IsSettling;

    /// <summary>Whether a check of a selection or a declaration of blockers has been sent and its verdict has not come back.</summary>
    public bool IsChecking => _checking.Count > 0 || _blocks.IsChecking || _attacks.IsChecking;

    /// <summary>
    /// An answer to a choice of cards on its way to being one the rules take: the cards proposed, checked as they are; and if
    /// the host refuses them, a selection built a card at a time from what may be added, preferring the cards proposed and
    /// then the question's own order, going back a card where nothing more may be added and the cards are no answer.
    /// </summary>
    private sealed class Settling(PromptMessage asked, IReadOnlyList<ViewHandle> proposal)
    {
        public PromptMessage Asked { get; } = asked;

        public List<ViewHandle> Proposal { get; } = [.. proposal];

        /// <summary>For a choice of targets, the players among the targets proposed: sent as they are, the rule judging only objects.</summary>
        public List<TargetMessage> Players { get; init; } = [];

        public bool Mending { get; set; }

        public List<ViewHandle> Picked { get; } = [];

        /// <summary>For each card picked and the next, the cards already tried there.</summary>
        public List<HashSet<int>> Tried { get; } = [[]];

        /// <summary>How many selections have been judged, against a choice whose rule nothing satisfies.</summary>
        public int Steps { get; set; }

        public IEnumerable<ViewHandle> Preference =>
            Proposal.Concat(((ISelectionPrompt)Asked.Prompt).Candidates.Where(card => !Proposal.Contains(card)));
    }

    /// <summary>The most selections settling a choice judges before it sends what it has and leaves the rest to the host.</summary>
    private const int SettlingSteps = 400;

    /// <summary>Takes the answer being settled as far as the verdicts in hand allow: sends it, or asks for the next verdict.</summary>
    private void Settle()
    {
        while (_settling is { } settling)
        {
            if (!settling.Mending)
            {
                SelectionVerdictView? judged = Verdict(settling.Proposal);
                if (judged is null)
                {
                    Check(settling.Proposal);
                    return;
                }

                if (judged.Complete)
                {
                    Send(settling.Proposal);
                    return;
                }

                settling.Mending = true;
                Mended($"request {settling.Asked.RequestId} ({settling.Asked.Prompt.GetType().Name}): the rules refuse the proposal {Cards(settling.Proposal)}; mending it from what may be added");
                continue;
            }

            SelectionVerdictView? verdict = Verdict(settling.Picked);
            if (verdict is null)
            {
                Check(settling.Picked);
                return;
            }

            if (++settling.Steps > SettlingSteps)
            {
                Send(settling.Picked);
                return;
            }

            HashSet<int> tried = settling.Tried[^1];
            bool Offered(ViewHandle card) =>
                !settling.Picked.Contains(card) && !tried.Contains(card.Value) && verdict.Addable.Contains(card);

            // The cards proposed come first, in the order proposed. Cards the proposal never named are added only to reach an
            // answer: once the cards picked are one, an optional choice is not grown into a card its player never picked.
            ViewHandle? next = settling.Proposal.Where(Offered).Select(card => (ViewHandle?)card).FirstOrDefault();
            if (next is null && !verdict.Complete)
            {
                next = settling.Preference.Where(Offered).Select(card => (ViewHandle?)card).FirstOrDefault();
            }

            if (next is { } card)
            {
                tried.Add(card.Value);
                settling.Picked.Add(card);
                settling.Tried.Add([]);
                continue;
            }

            if (verdict.Complete || settling.Picked.Count == 0)
            {
                // The most that could be added is an answer. Or nothing is: a question asked while a spell or ability is
                // announced is given up with it (a total power to tap out of reach once its mana is paid, CR 601.2h), the
                // one answer the rules take; any other question's host is left to refuse what was meant.
                if (!verdict.Complete && settling.Asked.MayCancel)
                {
                    Cancel();
                    return;
                }

                Send(verdict.Complete ? settling.Picked : settling.Proposal);
                return;
            }

            settling.Picked.RemoveAt(settling.Picked.Count - 1);
            settling.Tried.RemoveAt(settling.Tried.Count - 1);
        }

        void Send(IReadOnlyList<ViewHandle> cards)
        {
            PromptMessage asked = _settling!.Asked;
            if (_settling.Mending)
            {
                Mended($"request {asked.RequestId} ({asked.Prompt.GetType().Name}): proposed {Cards(_settling.Proposal)}, sent {Cards(cards)}");
            }

            List<TargetMessage> players = _settling.Players;
            _settling = null;
            LastAnswered = asked.RequestId;
            AnswerPayload sent = asked.Prompt is ChooseTargetPromptMessage
                ? new TargetsChosenAnswer { Targets = [.. players, .. cards.Select(card => new TargetMessage { Object = card })] }
                : new CardsChosenAnswer { Cards = [.. cards] };
            _transport.Send(new AnswerMessage { RequestId = asked.RequestId, Answer = sent });
        }

        void Cancel()
        {
            PromptMessage asked = _settling!.Asked;
            Mended($"request {asked.RequestId} ({asked.Prompt.GetType().Name}): no selection is an answer; proposed {Cards(_settling.Proposal)}, the announcement given up");
            _settling = null;
            LastAnswered = asked.RequestId;
            _transport.Send(new AnswerMessage { RequestId = asked.RequestId, Answer = new CancelAnnouncementAnswer() });
        }
    }

    /// <summary>The questions a check of a choice of cards was refused for.</summary>
    private readonly HashSet<int> _refusedSelections = [];

    /// <summary>
    /// The host refused a check of a choice of cards being settled. Nothing unsettled is ever sent: a question no longer
    /// outstanding, or not this seat's, has nothing to answer; otherwise the cards are not sent and the question is asked of
    /// this client again, the error kept in <see cref="Errors"/>, for its player to choose anew - and answered with its
    /// default if it is answered so again.
    /// </summary>
    private void RefusedSelectionCheck(Settling settling, ErrorMessage error)
    {
        _checking.Clear();
        _settling = null;
        _refusedSelections.Add(settling.Asked.RequestId);
        Mended($"a check of request {settling.Asked.RequestId} was refused ({error.Code}: {error.Detail}); the proposal {Cards(settling.Proposal)} is not sent");
        if (error.Code is not (ErrorCode.StaleReply or ErrorCode.NotYourTurnToAnswer) && Table?.Phase != TablePhase.Abandoned)
        {
            Prompt = settling.Asked;
        }
    }

    // --- Declarations of blockers and of attackers, judged by the host as they are made. ---

    /// <summary>
    /// One kind of declaration - of blockers or of attackers - judged by the host as it is made (CR 508.1a-d, 509.1a-d): the
    /// host's verdicts on the declarations asked about, and an answer on its way to being one the rules take. A declaration
    /// the host refuses is mended an item at a time from what may be added - the items meant first, then the engine's own
    /// declaration's, then any - and an addable item is one some legal declaration has, so adding them never runs into a dead
    /// end. A check refused never lets an unsettled declaration go: a question no longer outstanding (or not this seat's) is
    /// dropped, any other is asked again; and a question whose check was refused before is answered with the engine's own
    /// declaration (<c>Suggested</c>), which is legal, rather than the same proposal again, so an automatic seat cannot go
    /// round forever.
    /// </summary>
    private sealed class Declarations<TItem, TVerdict>(GameClient client, Declarations<TItem, TVerdict>.Kind kind)
        where TItem : class
        where TVerdict : class
    {
        /// <summary>What a kind of declaration is made of, and how its prompt, verdicts and messages read.</summary>
        public sealed record Kind(
            string Name,
            Func<PromptPayload, bool> Asks,
            Func<PromptPayload, TVerdict> NoneVerdict,
            Func<PromptPayload, List<TItem>> Suggested,
            Func<TItem, string> Key,
            Func<TVerdict, bool> Complete,
            Func<TVerdict, List<TItem>> Addable,
            Func<TVerdict, string> Why,
            Func<int, List<TItem>, ClientMessage> CheckMessage,
            Func<List<TItem>, AnswerPayload> Answer,
            Func<IEnumerable<TItem>, string> Describe,
            Func<PromptPayload, IReadOnlyList<TItem>, TVerdict?> Known);

        private readonly Dictionary<string, TVerdict> _verdicts = new(StringComparer.Ordinal);
        private readonly HashSet<string> _checking = new(StringComparer.Ordinal);
        private readonly HashSet<int> _refused = [];
        private int _verdictsFor;
        private Settling? _settling;

        private sealed class Settling(PromptMessage asked, IReadOnlyList<TItem> proposal)
        {
            public PromptMessage Asked { get; } = asked;
            public List<TItem> Proposal { get; } = [.. proposal];
            public bool Mending { get; set; }
            public List<TItem> Picked { get; } = [];
        }

        public bool IsSettling => _settling is not null;

        public bool IsChecking => _checking.Count > 0;

        public int? SettlingFor => _settling?.Asked.RequestId;

        private string KeyOf(IEnumerable<TItem> items) => string.Join(',', items.Select(kind.Key).Order(StringComparer.Ordinal));

        private bool Same(TItem a, TItem b) => kind.Key(a) == kind.Key(b);

        private PromptMessage? Asked => client.Prompt is { } asked && kind.Asks(asked.Prompt) ? asked : _settling?.Asked;

        /// <summary>The host's verdict on <paramref name="items"/>: the prompt's own for none, else once a check has come back.</summary>
        public TVerdict? Verdict(IEnumerable<TItem> items)
        {
            if (Asked is not { } asked) return null;
            List<TItem> declared = [.. items];
            if (declared.Count == 0) return kind.NoneVerdict(asked.Prompt);
            if (kind.Known(asked.Prompt, declared) is { } known) return known;
            return _verdictsFor == asked.RequestId && _verdicts.TryGetValue(KeyOf(declared), out TVerdict? verdict) ? verdict : null;
        }

        /// <summary>Asks the host's verdict on <paramref name="items"/>, unless it is known or being asked.</summary>
        public void Check(IEnumerable<TItem> items)
        {
            if (Asked is not { } asked) return;
            List<TItem> declared = [.. items];
            if (Verdict(declared) is not null) return;
            if (_verdictsFor != asked.RequestId)
            {
                _verdicts.Clear();
                _checking.Clear();
                _verdictsFor = asked.RequestId;
            }

            if (!_checking.Add(KeyOf(declared))) return;
            client._transport.Send(kind.CheckMessage(asked.RequestId, declared));
        }

        public void Note(int requestId, IEnumerable<TItem> items, TVerdict verdict)
        {
            if (requestId != _verdictsFor) return;
            string key = KeyOf(items);
            _checking.Remove(key);
            _verdicts[key] = verdict;
            client.VerdictRevision++;
        }

        /// <summary>
        /// Takes an answer to <paramref name="asked"/> over where it is not known to be legal: settles it with the host, or
        /// for a question whose check was refused before, sends the engine's own declaration. False where it may go as it is.
        /// </summary>
        public bool Takes(PromptMessage asked, IReadOnlyList<TItem> proposal)
        {
            if (Verdict(proposal) is { } known && kind.Complete(known)) return false;

            // Cleared before anything is sent, as for any answer.
            client.Prompt = null;
            if (_refused.Contains(asked.RequestId))
            {
                List<TItem> suggested = kind.Suggested(asked.Prompt);
                client.Mended($"request {asked.RequestId} ({kind.Name}): a check of it was refused before; sending the engine's own declaration {kind.Describe(suggested)} for {kind.Describe(proposal)}");
                Send(asked, suggested);
                return true;
            }

            _settling = new Settling(asked, proposal);
            Settle();
            return true;
        }

        /// <summary>Takes the declaration being settled as far as the verdicts in hand allow: sends it, or asks for the next verdict.</summary>
        public void Settle()
        {
            while (_settling is { } settling)
            {
                if (!settling.Mending)
                {
                    TVerdict? judged = Verdict(settling.Proposal);
                    if (judged is null)
                    {
                        Check(settling.Proposal);
                        return;
                    }

                    if (kind.Complete(judged))
                    {
                        Send(settling.Asked, settling.Proposal);
                        return;
                    }

                    settling.Mending = true;
                    client.Mended($"request {settling.Asked.RequestId} ({kind.Name}): the rules refuse the declaration {kind.Describe(settling.Proposal)} ({kind.Why(judged)}); mending it from what may be added");
                    continue;
                }

                TVerdict? verdict = Verdict(settling.Picked);
                if (verdict is null)
                {
                    Check(settling.Picked);
                    return;
                }

                List<TItem> addable = kind.Addable(verdict);
                bool Offered(TItem item) => addable.Exists(other => Same(other, item));

                // The items proposed come first; others only to reach a legal declaration, the engine's own first.
                TItem? next = settling.Proposal.FirstOrDefault(Offered);
                if (next is null && !kind.Complete(verdict))
                {
                    next = kind.Suggested(settling.Asked.Prompt).FirstOrDefault(Offered) ?? addable.FirstOrDefault();
                }

                if (next is not null && settling.Picked.Count <= settling.Proposal.Count + kind.Suggested(settling.Asked.Prompt).Count + addable.Count)
                {
                    settling.Picked.Add(next);
                    continue;
                }

                Send(settling.Asked, kind.Complete(verdict) ? settling.Picked : kind.Suggested(settling.Asked.Prompt));
                return;
            }
        }

        private void Send(PromptMessage asked, IReadOnlyList<TItem> items)
        {
            if (_settling is { Mending: true } mending)
            {
                client.Mended($"request {asked.RequestId} ({kind.Name}): proposed {kind.Describe(mending.Proposal)}, sent {kind.Describe(items)}");
            }

            _settling = null;
            client.LastAnswered = asked.RequestId;
            client._transport.Send(new AnswerMessage { RequestId = asked.RequestId, Answer = kind.Answer([.. items]) });
        }

        /// <summary>
        /// The host refused a check of <paramref name="error"/>'s request while it was being settled: nothing unsettled is
        /// sent. A question no longer outstanding, or not this seat's, has nothing to answer; otherwise it is asked of this
        /// client again, and answered with the engine's own declaration if it is answered so again.
        /// </summary>
        public void Refused(ErrorMessage error)
        {
            if (_settling is not { } settling || _checking.Count == 0 || error.RequestId != settling.Asked.RequestId) return;
            _checking.Clear();
            _settling = null;
            _refused.Add(settling.Asked.RequestId);
            client.Mended($"a check of request {settling.Asked.RequestId} was refused ({error.Code}: {error.Detail}); the declaration {kind.Describe(settling.Proposal)} is not sent");
            if (error.Code is not (ErrorCode.StaleReply or ErrorCode.NotYourTurnToAnswer) && client.Table?.Phase != TablePhase.Abandoned)
            {
                client.Prompt = settling.Asked;
            }
        }

        /// <summary>Forgets the answer being settled: a new question was asked, or the game ended.</summary>
        public void Reset() => _settling = null;
    }

    private static string BlockKey(BlockMessage block) => $"{block.Blocker.Value}>{block.Attacker.Value}";

    private static string AttackKey(AttackMessage attack) => $"{attack.Attacker.Value}>{attack.TargetPlayer}/{attack.TargetPermanent.Value}";

    private Declarations<BlockMessage, BlockVerdictView>? _blocksField;

    private Declarations<BlockMessage, BlockVerdictView> _blocks => _blocksField ??= new(this, new(
        nameof(DeclareBlockersPromptMessage),
        prompt => prompt is DeclareBlockersPromptMessage,
        prompt => ((DeclareBlockersPromptMessage)prompt).Verdict,
        prompt => [.. ((DeclareBlockersPromptMessage)prompt).Suggested],
        BlockKey,
        verdict => verdict.Complete,
        verdict => verdict.Addable,
        verdict => verdict.Issue is { } issue ? $"{issue.Words}, CR {issue.Rule}" : string.Empty,
        (request, blocks) => new CheckBlocksMessage { RequestId = request, Blocks = blocks },
        blocks => new BlockersDeclaredAnswer { Blocks = blocks },
        blocks => "[" + string.Join(", ", blocks.Select(block => $"{Cards([block.Blocker])} > {Cards([block.Attacker])}")) + "]",
        (_, _) => null));

    private Declarations<AttackMessage, AttackVerdictView>? _attacksField;

    private Declarations<AttackMessage, AttackVerdictView> _attacks => _attacksField ??= new(this, new(
        nameof(DeclareAttackersPromptMessage),
        prompt => prompt is DeclareAttackersPromptMessage,
        prompt => ((DeclareAttackersPromptMessage)prompt).Verdict,
        prompt => [.. ((DeclareAttackersPromptMessage)prompt).Suggested],
        AttackKey,
        verdict => verdict.Complete,
        verdict => verdict.Addable,
        verdict => verdict.Issue is { } issue ? $"{issue.Words}, CR {issue.Rule}" : string.Empty,
        (request, attacks) => new CheckAttacksMessage { RequestId = request, Attacks = attacks },
        attacks => new AttackersDeclaredAnswer { Attacks = attacks },
        attacks => "[" + string.Join(", ", attacks.Select(attack => $"{Cards([attack.Attacker])} > {(attack.TargetPermanent.IsNone ? $"seat {attack.TargetPlayer}" : Cards([attack.TargetPermanent]))}")) + "]",
        // Where the host says nothing is required, a declaration of attacks it offers, each creature once, is legal as it is,
        // and costs what its attacks' listed costs add up to: known without asking.
        (prompt, attacks) => prompt is DeclareAttackersPromptMessage { Verdict.Free: true } asked
            && attacks.Select(attack => attack.Attacker).Distinct().Count() == attacks.Count
            && attacks.All(attack => asked.Verdict.Addable.Exists(offered => AttackKey(offered) == AttackKey(attack)))
                ? new AttackVerdictView
                {
                    Complete = true,
                    Free = true,
                    Addable = asked.Verdict.Addable.FindAll(offered => !attacks.Any(attack => attack.Attacker == offered.Attacker)),
                    Cost = asked.TotalCostText(attacks),
                }
                : null));

    /// <summary>
    /// What the rules made of the declaration of blockers <paramref name="blocks"/> for the declaration outstanding (CR
    /// 509.1a-d): whether it may be declared, why not, and which blocks may be added. For no blocks it is the prompt's own
    /// <see cref="DeclareBlockersPromptMessage.Verdict"/>; any other declaration is known once the host has answered a
    /// <see cref="CheckBlocks"/> of it, and null until then (or when no declaration is asked).
    /// </summary>
    public BlockVerdictView? BlockVerdict(IEnumerable<BlockMessage> blocks)
    {
        ArgumentNullException.ThrowIfNull(blocks);
        return _blocks.Verdict(blocks);
    }

    /// <summary>
    /// Asks the host what the rules make of <paramref name="blocks"/> for the outstanding declaration of blockers
    /// (docs/host-protocol.md, "Checking a declaration of blockers"): its <see cref="BlocksVerdictMessage"/> arrives with a
    /// later <see cref="Pump"/>, and <see cref="BlockVerdict"/> reads it. A declaration already judged, or being judged, is
    /// not asked again.
    /// </summary>
    public void CheckBlocks(IEnumerable<BlockMessage> blocks)
    {
        ArgumentNullException.ThrowIfNull(blocks);
        _blocks.Check(blocks);
    }

    /// <summary>
    /// What the rules made of the declaration of attackers <paramref name="attacks"/> for the declaration outstanding (CR
    /// 508.1a-d): whether it may be declared, why not, and which attacks may be added. For none it is the prompt's own
    /// <see cref="DeclareAttackersPromptMessage.Verdict"/>; any other is known once the host has answered a
    /// <see cref="CheckAttacks"/> of it, and null until then.
    /// </summary>
    public AttackVerdictView? AttackVerdict(IEnumerable<AttackMessage> attacks)
    {
        ArgumentNullException.ThrowIfNull(attacks);
        return _attacks.Verdict(attacks);
    }

    /// <summary>
    /// Asks the host what the rules make of <paramref name="attacks"/> for the outstanding declaration of attackers
    /// (docs/host-protocol.md, "Checking a declaration of attackers"); <see cref="AttackVerdict"/> reads it once it is back.
    /// </summary>
    public void CheckAttacks(IEnumerable<AttackMessage> attacks)
    {
        ArgumentNullException.ThrowIfNull(attacks);
        _attacks.Check(attacks);
    }

    public void Discard(IEnumerable<ViewHandle> cards) =>
        Answer(new CardsChosenAnswer { Cards = [.. cards] });

    // --- Reading the view. Convenience only: no interpretation, no rules. ---

    public PlayerSummaryView? Me =>
        View?.Players.FirstOrDefault(p => p.Seat == Seat);

    public IEnumerable<PlayerSummaryView> Opponents =>
        View?.Players.Where(p => p.Seat != Seat) ?? [];

    public ObjectView? Object(ViewHandle handle)
    {
        if (View is not { } view)
        {
            return null;
        }

        // Indexed once per view rather than searched per call: a library search looks up every one of hundreds of cards,
        // and a search per card made that hundreds of thousands of comparisons. The first object under a handle wins, as
        // the search it replaces found it.
        Index(view);
        return _byHandle.GetValueOrDefault(handle);
    }

    private readonly Dictionary<ViewHandle, ObjectView> _byHandle = [];
    private readonly Dictionary<(ZoneKindView Kind, int Owner), IReadOnlyList<ObjectView>> _byZone = [];
    private PlayerView? _indexed;

    /// <summary>
    /// Puts back, in a view just received, every object the last view had unchanged - the same handle with the same
    /// fingerprint (<see cref="ObjectView.Fingerprint"/>) - so that whatever was worked out from an object and kept with it
    /// holds for as long as the object does not change, rather than for one view. A board of two hundred permanents where
    /// one card tapped is one new object, not two hundred.
    /// </summary>
    private void KeepUnchanged(PlayerView next)
    {
        if (View is not { } last)
        {
            return;
        }

        Index(last);
        for (int i = 0; i < next.Objects.Count; i++)
        {
            ObjectView received = next.Objects[i];
            if (received.Fingerprint != 0
                && _byHandle.TryGetValue(received.Handle, out ObjectView? kept)
                && kept.Fingerprint == received.Fingerprint)
            {
                next.Objects[i] = kept;
            }
        }
    }

    /// <summary>Starts this view's indexes over when it is a new view: a view is never changed once received.</summary>
    private void Index(PlayerView view)
    {
        if (ReferenceEquals(_indexed, view))
        {
            return;
        }

        _byHandle.Clear();
        _byZone.Clear();
        foreach (ObjectView item in view.Objects)
        {
            _byHandle.TryAdd(item.Handle, item);
        }

        _indexed = view;
    }

    /// <summary>
    /// Cards in a zone, in the order the server sent them. Empty for a zone this client cannot see into.
    /// <para>
    /// Listed once per view and handed out again after that: a redraw asks for the battlefield once for every permanent
    /// on it, and a list per ask made a board of two hundred permanents cost forty thousand lookups a redraw.
    /// </para>
    /// </summary>
    public IReadOnlyList<ObjectView> Zone(ZoneKindView kind, int owner)
    {
        if (View is not { } view)
        {
            return [];
        }

        Index(view);
        if (_byZone.TryGetValue((kind, owner), out IReadOnlyList<ObjectView>? listed))
        {
            return listed;
        }

        ZoneView? zone = view.Zones.FirstOrDefault(z => z.Kind == kind && z.Owner == owner);
        List<ObjectView> contents = [];
        if (zone is { ContentsVisible: true })
        {
            foreach (ViewHandle handle in zone.Contents)
            {
                if (_byHandle.GetValueOrDefault(handle) is { } card)
                {
                    contents.Add(card);
                }
            }
        }

        _byZone[(kind, owner)] = contents;
        return contents;
    }

    /// <summary>
    /// Cards this client knows in a zone it cannot see into, such as what is left of a hand Duress revealed. A hand's come
    /// in name order, a library's top first. Empty for a zone it can see into, where <see cref="Zone"/> has everything.
    /// </summary>
    public IReadOnlyList<KnownCardView> KnownCards(ZoneKindView kind, int owner) =>
        View?.Zones.FirstOrDefault(z => z.Kind == kind && z.Owner == owner)?.Known ?? [];

    /// <summary>
    /// Every card this client can name in a zone it cannot see into: the ones it remembers and the ones it is being
    /// shown this moment, in name order so nothing about the order says which card is which. Right for a hand, where
    /// the cards are not held by position; a library's are, and <see cref="Shown"/> says which of those is its top.
    /// <para>
    /// The two arrive by different channels and a card crosses between them as a reveal comes and goes: while it is
    /// being revealed it is in the view with a handle, and from then on it is remembered without one. Reading only one
    /// of them turns a card face down for exactly the message that announces it was revealed.
    /// </para>
    /// </summary>
    public IReadOnlyList<ObjectView> NamedIn(ZoneKindView kind, int owner) =>
        [.. KnownCards(kind, owner).Select(known => known.Card).Concat(Shown(kind, owner))
            .OrderBy(card => card.Name, StringComparer.Ordinal)
            .ThenBy(card => card.CardId, StringComparer.Ordinal)];

    /// <summary>
    /// Cards this client is being shown, with their handles, in a zone it cannot see into: a library being searched, or
    /// the top of a library under Bolas's Citadel. In id order, which says nothing about where each one is. Empty for a
    /// zone it can see into.
    /// </summary>
    public IReadOnlyList<ObjectView> Shown(ZoneKindView kind, int owner) =>
        View is null || View.Zones.FirstOrDefault(z => z.Kind == kind && z.Owner == owner) is not { ContentsVisible: false }
            ? []
            : [.. View.Objects.Where(o => o.Zone == kind && o.ZoneOwner == owner)];

    /// <summary>How many cards are in a zone, whether or not this client can see into it.</summary>
    public int ZoneCount(ZoneKindView kind, int owner) =>
        View?.Zones.FirstOrDefault(z => z.Kind == kind && z.Owner == owner)?.Count ?? 0;

    public IReadOnlyList<ObjectView> MyHand() => Zone(ZoneKindView.Hand, Seat);

    public IReadOnlyList<ObjectView> Battlefield() => Zone(ZoneKindView.Battlefield, 0);

    /// <summary>
    /// The permanents a player controls that the game treats as existing: a phased-out one is shown with the battlefield
    /// (<see cref="Battlefield"/>) but can't attack, block, be sacrificed or do anything until it phases in (CR 702.26b).
    /// </summary>
    public IEnumerable<ObjectView> BattlefieldOf(int seat) =>
        Battlefield().Where(o => o.Controller == seat && !o.PhasedOut);

    /// <summary>
    /// The permanents a player controls as a player sees them: <see cref="BattlefieldOf"/> and the phased-out ones too, which
    /// the game treats as though they don't exist but does not hide (CR 702.26b) - what a table draws and a narration names.
    /// </summary>
    public IEnumerable<ObjectView> BattlefieldShownOf(int seat) =>
        Battlefield().Where(o => o.Controller == seat);

    /// <summary>
    /// The game log as the server has written it for this seat, oldest first. The server keeps it and sends all of it
    /// again when this client is seated or resumed, so it is complete however long this client was away.
    /// </summary>
    public IReadOnlyList<LogEntryView> Log => _log;

    /// <summary>Bumped whenever lines are taken away or replaced rather than added, so a view that appends knows to start over.</summary>
    public int LogRevision { get; private set; }

    private readonly List<LogEntryView> _log = [];

    private void ApplyLog(LogMessage log)
    {
        if (log.Complete)
        {
            _log.Clear();
            LogRevision++;
        }

        if (log.Removed.Count > 0)
        {
            HashSet<long> gone = [.. log.Removed];
            if (_log.RemoveAll(entry => gone.Contains(entry.Number)) > 0)
            {
                LogRevision++;
            }
        }

        _log.AddRange(log.Entries);
    }

    private void Apply(ServerMessage message)
    {
        switch (message)
        {
            case MembershipMessage membership:
                Membership = membership;
                Seat = membership.Seat;
                break;

            case TableMessage table:
                Table = table;
                if (table.Phase != TablePhase.Drafting)
                {
                    Draft = null;
                }

                if (table.Phase != TablePhase.DeckBuilding)
                {
                    DeckBuilding = null;
                }

                if (table.Phase != TablePhase.Pairings)
                {
                    Pairings = null;
                }

                if (table.Phase == TablePhase.Abandoned)
                {
                    Prompt = null;
                }
                break;

            case WelcomeMessage welcome:
                Seat = welcome.Seat;
                PlayerNames = welcome.PlayerNames;
                Decklist = welcome.Decklist;
                foreach (DeckCardView entry in welcome.Decklist)
                {
                    NotePrinted(entry.Card);
                }

                break;

            case StateMessage state:
                foreach (ObjectView printed in state.Printed)
                {
                    NotePrinted(printed);
                }

                KeepUnchanged(state.View);
                View = state.View;

                // A seat out of a game that goes on - it lost, conceded or ran out of time (CR 800.4a) - is asked nothing more.
                if (state.View.Players.FirstOrDefault(player => player.Seat == Seat) is { Status: not PlayerStatusView.Active })
                {
                    Prompt = null;
                }

                break;

            case PromptMessage prompt:
                // The question an answer is being settled for, asked again (a resumed connection): the answer is still on its way.
                if (_settling?.Asked.RequestId == prompt.RequestId || _blocks.SettlingFor == prompt.RequestId || _attacks.SettlingFor == prompt.RequestId) break;
                _settling = null;
                _blocks.Reset();
                _attacks.Reset();
                if (Table?.Phase != TablePhase.Abandoned) Prompt = prompt;
                break;

            case SelectionVerdictMessage verdict:
                Note(verdict);
                break;

            case BlocksVerdictMessage verdict:
                _blocks.Note(verdict.RequestId, verdict.Blocks, verdict.Verdict);
                break;

            case AttacksVerdictMessage verdict:
                _attacks.Note(verdict.RequestId, verdict.Attacks, verdict.Verdict);
                break;

            case GameOverMessage over:
                Outcome = over;
                Prompt = null;
                _settling = null;
                _blocks.Reset();
                _attacks.Reset();
                DrawOffer = null;
                break;

            case ErrorMessage error:
                _errors.Add(error);

                // The check of this answer's own question refused while it was being settled (the error names the request it is
                // about; any other error is not this answer's): nothing unsettled is sent.
                if (_settling is { } settling && _checking.Count > 0 && error.RequestId == settling.Asked.RequestId)
                {
                    RefusedSelectionCheck(settling, error);
                }

                _blocks.Refused(error);
                _attacks.Refused(error);

                // An answer to a draw refused leaves the seat free to answer again.
                _drawAnswerSent = false;
                _acceptAfterDecline = false;
                break;

            case LogMessage log:
                ApplyLog(log);
                break;

            case DraftMessage draft:
                Draft = draft;
                break;

            case SeatReleasedMessage released:
                Released = released;
                Prompt = null;
                break;

            case TableMoveMessage moved:
                Moved = moved;
                Prompt = null;
                break;

            case DeckBuildingMessage building:
                DeckBuilding = building;
                break;

            case PairingsMessage pairings:
                Pairings = pairings;
                break;

            case ClockMessage clock:
                Clock = clock;
                ClockReceived = DateTime.UtcNow;
                break;

            case DrawOfferMessage offer:
                DrawOffer = offer.Offerer == 0 ? null : offer;

                // The answer sent has landed when this seat is among those who answered, or there is no offer any more.
                // A seat that declined and then accepts is among them already, so it waits for the acceptance itself.
                if (offer.Offerer == 0 || offer.Accepted.Contains(Seat) || (offer.Declined.Contains(Seat) && !_acceptAfterDecline))
                {
                    _drawAnswerSent = false;
                    _acceptAfterDecline = false;
                }

                break;

            default:
                // An unrecognised message from a newer server is not a reason to fall over; the handshake is
                // what catches a real mismatch.
                break;
        }
    }
}
