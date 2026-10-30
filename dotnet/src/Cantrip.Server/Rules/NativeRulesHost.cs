using System.Text.Json;
using System.Text.Json.Nodes;
using Cantrip.Protocol;

namespace Cantrip.Server.Rules;

/// <summary>
/// The native Rust engine behind <see cref="IRulesHost"/>: one game in the <c>cantrip-native-host</c> sidecar, driven
/// over an <see cref="IHostTransport"/> with the requests and replies of the host's JSON-lines protocol
/// (<c>crates/cantrip-host</c>).
/// <para>
/// Translates nothing. The host emits <c>Cantrip.Protocol</c>'s own types, already projected for each seat and in
/// each seat's own handle space, so this reads them (<see cref="ProtocolJson"/>) and forwards them. What is kept here
/// is only what the session asks between steps — the decklists for the welcome, the latest views for who may concede,
/// the cards asked for as printed — and the answers given, which is how the game is resumed: a fresh deal from the same
/// seed and decks, and the answers sent again.
/// </para>
/// <para>
/// Under this host a view's <see cref="ObjectView.CardId"/> is the card's name: the engine knows cards by name, and
/// the client already finds art by the slug of the face's name.
/// </para>
/// </summary>
public sealed class NativeRulesHost : IRulesHost
{
    private readonly IHostTransport _transport;
    private readonly IReadOnlyList<RulesSeat> _seats;
    private readonly List<int> _seatNumbers;
    private readonly List<string> _names;
    private readonly IReadOnlyList<RulesAnswer> _resume;
    private readonly Dictionary<int, List<DeckCardView>> _decklists = [];
    private readonly Dictionary<string, ObjectView?> _oracles = new(StringComparer.Ordinal);
    private readonly List<RulesAnswer> _history = [];

    /// <summary>Each seat's view as of the last step, for the questions a session asks between steps.</summary>
    private IReadOnlyDictionary<int, PlayerView> _views = new Dictionary<int, PlayerView>();

    private bool _started;
    private bool _finished;
    private bool _closed;
    private GameOutcomeView _outcome;
    private int _winningTeam;

    /// <summary>
    /// How far the log has been brought: the host's own count when its events carry a <c>Sequence</c>, else events
    /// numbered here as they arrive.
    /// </summary>
    private long _sequence;

    private NativeRulesHost(IHostTransport transport, IReadOnlyList<RulesSeat> seats, IReadOnlyList<RulesAnswer> resume)
    {
        _transport = transport;
        _seats = seats;
        _resume = resume;
        _seatNumbers = [.. Enumerable.Range(1, seats.Count)];
        _names = [.. seats.Select(seat => seat.Name)];
    }

    /// <summary>The host's id for this game, in every request after the first.</summary>
    public int GameId { get; private set; }

    /// <summary>
    /// Deals a new game in the host, or deals one again and gives it <paramref name="history"/>'s answers once more
    /// when <see cref="Start"/> is called: a game is resumed the way it always was. With <paramref name="publicView"/>, every
    /// step also carries the game as a viewer who is no player sees it (<see cref="RulesStep.Public"/>); without it the host
    /// works none of that out.
    /// </summary>
    /// <exception cref="RulesRejectedException">The host would not deal it: a deck names a card it does not know.</exception>
    public static NativeRulesHost NewGame(
        IHostTransport transport,
        IReadOnlyList<RulesSeat> seats,
        ulong seed,
        RulesGameOptions options,
        IReadOnlyList<RulesAnswer>? history = null,
        bool publicView = false)
    {
        ArgumentNullException.ThrowIfNull(transport);
        ArgumentNullException.ThrowIfNull(seats);
        ArgumentNullException.ThrowIfNull(options);

        if (seats.Count == 0)
        {
            throw new ArgumentException("A game needs at least one seat.", nameof(seats));
        }

        var host = new NativeRulesHost(transport, seats, history ?? []);
        List<object> named = [.. seats.Select(seat => seat.DeckName is { } deckName
            ? (object)new { seat.Name, seat.Deck, seat.Sideboard, seat.Companion, DeckName = deckName }
            : new { seat.Name, seat.Deck, seat.Sideboard, seat.Companion })];
        object deal = options.Scenario is { } scenario
            ? new
            {
                Cmd = "NewGame",
                Seats = named,
                Seed = seed,
                Scenario = new { scenario.Name, scenario.Text, scenario.Driven },
            }
            : new
            {
                Cmd = "NewGame",
                Seats = named,
                Seed = seed,
                options.Shuffle,
                options.FirstPlayer,
                options.Chooser,
                options.BasicLandsOutside,
                options.MinimumDeckSize,
            };
        NewGameReply dealt = host.Send<NewGameReply>(publicView ? WithPublic(deal) : deal);

        host.GameId = dealt.GameId;
        foreach (int seat in host._seatNumbers)
        {
            host._decklists[seat] = Decklist(dealt.Decklists.TryGetValue(seat, out List<ObjectView>? cards) ? cards : []);
        }

        return host;
    }

    /// <summary>The request <paramref name="deal"/> with <c>"Public": true</c> beside what it says.</summary>
    private static JsonObject WithPublic(object deal)
    {
        JsonObject request = JsonSerializer.SerializeToNode(deal, ProtocolJson.Options)!.AsObject();
        request["Public"] = true;
        return request;
    }

    /// <summary>
    /// A deck as the welcome lists it: by name, with how many, in name order, each as printed. The host sends the deck
    /// card by card in deck order, which is the one thing a decklist must not say.
    /// </summary>
    private static List<DeckCardView> Decklist(IReadOnlyList<ObjectView> cards) =>
        [.. cards
            .GroupBy(card => card.CardId, StringComparer.Ordinal)
            .Select(copies => new DeckCardView { Name = copies.First().Name, Count = copies.Count(), Card = copies.First() })
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)];

    public IReadOnlyList<int> Seats => _seatNumbers;

    public IReadOnlyList<string> SeatNames => _names;

    /// <summary>A seat's team as the last view says; its own number before any view, which is every seat for itself.</summary>
    public int Team(int seat) => Summary(seat)?.Team ?? seat;

    public IReadOnlyList<DeckCardView> Decklist(int seat) =>
        _decklists.TryGetValue(seat, out List<DeckCardView>? deck) ? deck : [];

    public bool IsOver => _finished;

    public GameOutcomeView Outcome => _outcome;

    public int WinningTeam => _winningTeam;

    public IReadOnlyList<RulesAnswer> History => _history;

    public RulesStep Start()
    {
        if (_started)
        {
            throw new InvalidOperationException("The game has already started.");
        }

        _started = true;
        RulesStep step = Take(Send<StepReply>(new { Cmd = "Start", GameId }));
        if (_resume.Count == 0)
        {
            return step;
        }

        // The recorded answers given again, each to the question then outstanding for its seat. Everything that
        // happened along the way is published together with the position reached, as a live game
        // publishes it: a seat joining a resumed game reads the whole log, not just the last move.
        Dictionary<int, List<(long Sequence, GameEventView View)>> replayed = [];
        List<(long Sequence, GameEventView View)> replayedPublic = [];
        Gather(replayed, replayedPublic, step);
        foreach (RulesAnswer recorded in _resume)
        {
            RulesRequest asked = step.Requests.FirstOrDefault(request => request.Seat == recorded.Seat)
                ?? throw new InvalidOperationException(
                    $"The history answers for seat {recorded.Seat} where nothing was asked of it, so it does not replay.");

            try
            {
                step = Answer(asked.RequestId, recorded.Seat, recorded.Answer);
            }
            catch (RulesRejectedException refused)
            {
                throw new InvalidOperationException($"The history does not replay: {refused.Code}, {refused.Detail}");
            }

            Gather(replayed, replayedPublic, step);
        }

        return step with
        {
            Events = replayed.ToDictionary(
                seat => seat.Key,
                seat => (IReadOnlyList<(long Sequence, GameEventView View)>)seat.Value),
            Public = step.Public is { } reached ? reached with { Events = replayedPublic } : null,
        };
    }

    private static void Gather(
        Dictionary<int, List<(long Sequence, GameEventView View)>> into,
        List<(long Sequence, GameEventView View)> intoPublic,
        RulesStep step)
    {
        foreach ((int seat, IReadOnlyList<(long Sequence, GameEventView View)> events) in step.Events)
        {
            (into.TryGetValue(seat, out List<(long Sequence, GameEventView View)>? gathered) ? gathered : into[seat] = []).AddRange(events);
        }

        if (step.Public is { } shown)
        {
            intoPublic.AddRange(shown.Events);
        }
    }

    public RulesStep Answer(int requestId, int seat, AnswerPayload answer)
    {
        ArgumentNullException.ThrowIfNull(answer);

        RulesStep step = Take(Send<StepReply>(new { Cmd = "Answer", GameId, RequestId = requestId, Seat = seat, Answer = answer }));
        _history.Add(new RulesAnswer(seat, answer));
        return step;
    }

    public SelectionVerdictView CheckSelection(int requestId, int seat, IReadOnlyList<ViewHandle> selected)
    {
        ArgumentNullException.ThrowIfNull(selected);
        return Send<SelectionVerdictView>(new { Cmd = "Check", GameId, RequestId = requestId, Seat = seat, Selected = selected });
    }

    public BlockVerdictView CheckBlocks(int requestId, int seat, IReadOnlyList<BlockMessage> blocks)
    {
        ArgumentNullException.ThrowIfNull(blocks);
        return Send<BlockVerdictView>(new { Cmd = "CheckBlocks", GameId, RequestId = requestId, Seat = seat, Blocks = blocks });
    }

    public AttackVerdictView CheckAttacks(int requestId, int seat, IReadOnlyList<AttackMessage> attacks)
    {
        ArgumentNullException.ThrowIfNull(attacks);
        return Send<AttackVerdictView>(new { Cmd = "CheckAttacks", GameId, RequestId = requestId, Seat = seat, Attacks = attacks });
    }

    /// <summary>
    /// Only when conceding ends the game (CR 104.3a, 800.4a), read off the last view the way the rules decide it: the
    /// seat is in the game, and everyone else still in it is of one team.
    /// </summary>
    public bool CanConcede(int seat)
    {
        if (!_started || _finished || Summary(seat) is not { Status: PlayerStatusView.Active } me)
        {
            return false;
        }

        PlayerView view = _views.Values.First();
        return view.Players
            .Where(player => player.Seat != me.Seat && player.Status == PlayerStatusView.Active)
            .Select(player => player.Team)
            .Distinct()
            .Count() == 1;
    }

    public RulesStep Concede(int seat)
    {
        if (!CanConcede(seat))
        {
            throw new InvalidOperationException($"Seat {seat} cannot concede now.");
        }

        return Take(Send<StepReply>(new { Cmd = "Concede", GameId, Seat = seat }));
    }

    public RulesStep Leave(int seat)
    {
        if (!_started || _finished || !StillIn(seat))
        {
            throw new InvalidOperationException($"Seat {seat} is not in the game.");
        }

        // The host's concession (CR 104.3a): the player loses; the game ends if one player is left, and goes on otherwise.
        return Take(Send<StepReply>(new { Cmd = "Concede", GameId, Seat = seat }));
    }

    public RulesStep? End(int? winner)
    {
        if (!_started || _finished)
        {
            return null;
        }

        return Take(Send<StepReply>(new { Cmd = "End", GameId, Winner = winner }));
    }

    public PlayerView View(int seat)
    {
        if (!_started)
        {
            throw new InvalidOperationException("Only a started game has a view.");
        }

        return Send<ViewReply>(new { Cmd = "View", GameId, Seat = seat }).View
            ?? throw new InvalidOperationException($"The host sent no view for seat {seat}.");
    }

    public ObjectView? Oracle(string cardId)
    {
        ArgumentNullException.ThrowIfNull(cardId);

        if (!_oracles.TryGetValue(cardId, out ObjectView? card))
        {
            card = Send<OracleReply>(new { Cmd = "Oracle", Name = cardId }).Card;
            _oracles[cardId] = card;
        }

        return card;
    }

    public void Dispose()
    {
        if (_closed)
        {
            return;
        }

        _closed = true;
        try
        {
            Send<JsonElement>(new { Cmd = "Close", GameId });
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or JsonException or RulesRejectedException or ObjectDisposedException)
        {
            // Closing is a courtesy to a host that is still there; one that has gone has nothing to close.
        }
    }

    /// <summary>
    /// Whether a seat is still in the game, by every seat's view of the last step: a seat any of them shows out is out, since a
    /// seat not answering may be shown a position from before the run being played.
    /// </summary>
    private bool StillIn(int seat) => _views.Count > 0 && _views.Values.All(view =>
        view.Players.FirstOrDefault(player => player.Seat == seat) is not { } shown || shown.Status == PlayerStatusView.Active);

    private PlayerSummaryView? Summary(int seat) =>
        _views.Values.FirstOrDefault()?.Players.FirstOrDefault(player => player.Seat == seat);

    /// <summary>A step as the host sent it, with what this side keeps of it noted.</summary>
    private RulesStep Take(StepReply reply)
    {
        _views = reply.Views;
        _finished = reply.Finished;
        _outcome = reply.Outcome;
        _winningTeam = reply.WinningTeam;

        Dictionary<int, IReadOnlyList<(long Sequence, GameEventView View)>> events = [];
        foreach (int seat in _seatNumbers)
        {
            events[seat] = Numbered(reply.Events.TryGetValue(seat, out List<JsonElement>? batch) ? batch : []);
        }

        List<RulesRequest> requests = [];
        foreach (RequestReply request in reply.Requests)
        {
            PromptPayload prompt = request.Prompt
                ?? throw new InvalidOperationException($"Request {request.RequestId} carries no prompt.");

            requests.Add(new RulesRequest(
                request.RequestId,
                request.Seat,
                prompt,
                request.MayCancel,
                request.Presentation ?? PromptPresentation.Describe(prompt, request.MayCancel, _views.GetValueOrDefault(request.Seat)),
                request.ForSeat == request.Seat ? 0 : request.ForSeat));
        }

        PublicProjection? shown = reply.Public is { } projected ? Projection(projected) : null;
        return new RulesStep(requests, reply.Views, events, _sequence, reply.Finished, Progress(reply.Scenario), shown);
    }

    /// <summary>
    /// A batch of events as the host sent it, each with its number: the host's own when it gives one, so the same event is
    /// the same log line for every seat and for the public, and a retraction can name it; else counted here, which keeps the
    /// log in order and nothing more.
    /// </summary>
    private List<(long Sequence, GameEventView View)> Numbered(List<JsonElement> batch)
    {
        List<(long Sequence, GameEventView View)> numbered = [];
        foreach (JsonElement element in batch)
        {
            long sequence;
            if (element.ValueKind == JsonValueKind.Object
                && element.TryGetProperty("Sequence", out JsonElement given)
                && given.ValueKind == JsonValueKind.Number)
            {
                sequence = given.GetInt64();
                _sequence = Math.Max(_sequence, sequence);
            }
            else
            {
                sequence = ++_sequence;
            }

            GameEventView view = ProtocolJson.Deserialize<GameEventView>(element)
                ?? throw new InvalidOperationException("The host sent a null event.");

            numbered.Add((sequence, view));
        }

        return numbered;
    }

    /// <summary>
    /// A step's public part as the server keeps it: the view stamped with each object's fingerprint, as a seat's view is
    /// stamped before it is sent (<see cref="ObjectFingerprint"/>), its events numbered, and each seat's handle map.
    /// </summary>
    private PublicProjection Projection(PublicReply reply)
    {
        PlayerView view = reply.View ?? throw new InvalidOperationException("The host sent a public part with no view.");
        ObjectFingerprint.Stamp(view);
        Dictionary<int, IReadOnlyDictionary<ViewHandle, ViewHandle>> handles = [];
        foreach ((int seat, List<List<ViewHandle>> pairs) in reply.Handles)
        {
            Dictionary<ViewHandle, ViewHandle> map = [];
            foreach (List<ViewHandle> pair in pairs)
            {
                if (pair is not [ViewHandle shown, ViewHandle theirs])
                {
                    throw new InvalidOperationException($"The host sent a public handle pair of {pair.Count} for seat {seat}.");
                }

                map.Add(shown, theirs);
            }

            handles[seat] = map;
        }

        return new PublicProjection(view, Numbered(reply.Events), handles);
    }

    /// <summary>A step's scenario part as the server keeps it, or null for a dealt game.</summary>
    private static ScenarioProgress? Progress(ScenarioReply? reply)
    {
        if (reply is null)
        {
            return null;
        }

        ScenarioExpected? expected = reply.Expected is { } e
            ? new ScenarioExpected(
                e.Seat,
                e.RequestId,
                e.Answer is { ValueKind: JsonValueKind.Object } answer ? ProtocolJson.Deserialize<AnswerPayload>(answer) : null,
                e.Undo,
                e.Line)
            : null;
        ScenarioVerdict? verdict = reply.Verdict is { } v
            ? new ScenarioVerdict(v.Passed, v.Report, [.. v.Client.Select(Check)])
            : null;
        return new ScenarioProgress(expected, [.. reply.Checks.Select(Check)], verdict);

        static ScenarioCheck Check(CheckReply c) => new(c.Line, c.Key, c.Value);
    }

    /// <summary>
    /// The cards with companion in <paramref name="sideboard"/>, in its order, each with whether <paramref name="deck"/> - a
    /// starting deck, by card name - fulfils its condition (CR 702.139a, 103.2b), <paramref name="minimumDeckSize"/> being the
    /// format's minimum deck size (CR 100.2a): the engine's own judgement, for the deck editor, which has no rules of its own.
    /// </summary>
    /// <exception cref="RulesRejectedException">A card the engine does not have.</exception>
    public static IReadOnlyList<(string Card, bool Fulfilled)> Companions(
        IHostTransport transport, IReadOnlyList<string> deck, IReadOnlyList<string> sideboard, int minimumDeckSize)
    {
        ArgumentNullException.ThrowIfNull(transport);
        ArgumentNullException.ThrowIfNull(deck);
        ArgumentNullException.ThrowIfNull(sideboard);
        CompanionsReply reply = Exchange<CompanionsReply>(transport, new { Cmd = "Companions", Deck = deck, Sideboard = sideboard, MinimumDeckSize = minimumDeckSize });
        return [.. reply.Companions.Select(check => (check.Card, check.Fulfilled))];
    }

    /// <summary>
    /// One exchange: the request as a JSON line, the reply's <c>Ok</c> read as <typeparamref name="TReply"/>, or its
    /// <c>Rejected</c> thrown.
    /// </summary>
    private TReply Send<TReply>(object request) => Exchange<TReply>(_transport, request);

    private static TReply Exchange<TReply>(IHostTransport transport, object request)
    {
        string line = ProtocolJson.Serialize(request);
        string reply = transport.Exchange(line);

        using JsonDocument document = JsonDocument.Parse(reply);
        JsonElement root = document.RootElement;

        if (root.ValueKind == JsonValueKind.Object && root.TryGetProperty("Ok", out JsonElement ok))
        {
            return ProtocolJson.Deserialize<TReply>(ok)
                ?? throw new InvalidOperationException($"The host's reply to {Describe(request)} was empty: {reply}");
        }

        if (root.ValueKind == JsonValueKind.Object && root.TryGetProperty("Rejected", out JsonElement rejected))
        {
            string code = rejected.TryGetProperty("Code", out JsonElement codeElement) ? codeElement.GetString() ?? string.Empty : string.Empty;
            string detail = rejected.TryGetProperty("Detail", out JsonElement detailElement) ? detailElement.GetString() ?? string.Empty : string.Empty;

            throw new RulesRejectedException(
                Enum.TryParse(code, ignoreCase: false, out ErrorCode known) ? known : ErrorCode.IllegalAnswer,
                detail.Length > 0 ? detail : $"the host rejected {Describe(request)} with {code}.");
        }

        throw new InvalidOperationException($"The host's reply to {Describe(request)} is neither Ok nor Rejected: {reply}");
    }

    private static string Describe(object request) =>
        (request is JsonObject node ? node["Cmd"]?.ToString() : request.GetType().GetProperty("Cmd")?.GetValue(request)?.ToString()) ?? "a request";

    // --- The replies, as the host writes them. Settable, as everything ProtocolJson reads is. ---

    private sealed class NewGameReply
    {
        public int GameId { get; set; }

        public Dictionary<int, List<ObjectView>> Decklists { get; set; } = [];
    }

    private sealed class StepReply
    {
        public List<RequestReply> Requests { get; set; } = [];

        public Dictionary<int, PlayerView> Views { get; set; } = [];

        /// <summary>Each seat's events as elements, so a <c>Sequence</c> the host puts beside the event can be read first.</summary>
        public Dictionary<int, List<JsonElement>> Events { get; set; } = [];

        public bool Finished { get; set; }

        public GameOutcomeView Outcome { get; set; }

        public int WinningTeam { get; set; }

        /// <summary>A scenario's part of the step (docs/scenarios-in-the-browser.md), absent for a dealt game.</summary>
        public ScenarioReply? Scenario { get; set; }

        /// <summary>The public's part of the step (docs/host-protocol.md, "The public view"), absent unless dealt with one.</summary>
        public PublicReply? Public { get; set; }
    }

    private sealed class PublicReply
    {
        public PlayerView? View { get; set; }

        public List<JsonElement> Events { get; set; } = [];

        /// <summary>By seat, each pair of a public handle and that seat's handle for the same object.</summary>
        public Dictionary<int, List<List<ViewHandle>>> Handles { get; set; } = [];
    }

    private sealed class ScenarioReply
    {
        public ExpectedReply? Expected { get; set; }

        public List<CheckReply> Checks { get; set; } = [];

        public VerdictReply? Verdict { get; set; }
    }

    private sealed class ExpectedReply
    {
        public int Seat { get; set; }

        public int RequestId { get; set; }

        public JsonElement? Answer { get; set; }

        public bool Undo { get; set; }

        public string? Line { get; set; }
    }

    private sealed class CheckReply
    {
        public int Line { get; set; }

        public string Key { get; set; } = string.Empty;

        public string Value { get; set; } = string.Empty;
    }

    private sealed class VerdictReply
    {
        public bool Passed { get; set; }

        public string? Report { get; set; }

        public List<CheckReply> Client { get; set; } = [];
    }

    private sealed class RequestReply
    {
        public int RequestId { get; set; }

        public int Seat { get; set; }

        /// <summary>The seat whose decision it is: <see cref="Seat"/>, or a player that seat controls (CR 723.5).</summary>
        public int ForSeat { get; set; }

        public PromptPayload? Prompt { get; set; }

        public bool MayCancel { get; set; }

        /// <summary>Not in the protocol as laid out; read when a host sends it, worked out here when it does not.</summary>
        public PromptPresentationContext? Presentation { get; set; }
    }

    private sealed class ViewReply
    {
        public PlayerView? View { get; set; }
    }

    private sealed class OracleReply
    {
        public ObjectView? Card { get; set; }
    }

    private sealed class CompanionsReply
    {
        public List<CompanionReply> Companions { get; set; } = [];
    }

    private sealed class CompanionReply
    {
        public string Card { get; set; } = string.Empty;

        public bool Fulfilled { get; set; }
    }
}
