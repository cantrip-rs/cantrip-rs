using Cantrip.Protocol;
using Cantrip.Server.Rules;

namespace Cantrip.Server.InPerson;

/// <summary>What is told every step of a game dealt through an <see cref="ObservedRulesFactory"/>.</summary>
public interface IStepObserver
{
    /// <summary>
    /// A step the game took, on the table's own thread, before the table publishes it: <paramref name="started"/> for the
    /// opening (<see cref="IRulesHost.Start"/>), which begins a new game. Kept to a snapshot and a queue: the table waits on it.
    /// </summary>
    void Stepped(IRulesHost game, RulesStep step, bool started);

    /// <summary>The game's rules were let go: its table is done with it.</summary>
    void Closed(IRulesHost game);
}

/// <summary>
/// Deals each game through another factory and wraps it in an <see cref="ObservedRulesHost"/>, so that every step it
/// takes is shown to an observer - in-person play's relay - as well as to the table. The seam in-person play runs the
/// ordinary <c>GameServer</c> through (<c>GameServerOptions.Rules</c>), with no change to the table itself.
/// </summary>
public sealed class ObservedRulesFactory(IRulesHostFactory inner, IStepObserver observer) : IRulesHostFactory
{
    private readonly IRulesHostFactory _inner = inner ?? throw new ArgumentNullException(nameof(inner));
    private readonly IStepObserver _observer = observer ?? throw new ArgumentNullException(nameof(observer));

    public IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options) =>
        new ObservedRulesHost(_inner.Create(seats, seed, options), _observer);

    public bool CompanionFulfilled(IReadOnlyList<string> deck, string companion, int minimumDeckSize) =>
        _inner.CompanionFulfilled(deck, companion, minimumDeckSize);
}

/// <summary>
/// A rules host that is another, forwarding every member of <see cref="IRulesHost"/> to it unchanged - the ones the
/// interface gives a default too, so the table sees exactly the host it would have had - and handing each step it
/// returns to an <see cref="IStepObserver"/> on the way out. A test holds it to forwarding every member by reflection.
/// </summary>
public sealed class ObservedRulesHost : IRulesHost
{
    private readonly IRulesHost _inner;
    private readonly IStepObserver _observer;

    public ObservedRulesHost(IRulesHost inner, IStepObserver observer)
    {
        _inner = inner ?? throw new ArgumentNullException(nameof(inner));
        _observer = observer ?? throw new ArgumentNullException(nameof(observer));
    }

    /// <summary>The host this one forwards to.</summary>
    public IRulesHost Inner => _inner;

    public IReadOnlyList<int> Seats => _inner.Seats;

    public IReadOnlyList<string> SeatNames => _inner.SeatNames;

    public int Team(int seat) => _inner.Team(seat);

    public IReadOnlyList<DeckCardView> Decklist(int seat) => _inner.Decklist(seat);

    public RulesStep Start() => Observed(_inner.Start(), started: true);

    public RulesStep Answer(int requestId, int seat, AnswerPayload answer) => Observed(_inner.Answer(requestId, seat, answer));

    public SelectionVerdictView CheckSelection(int requestId, int seat, IReadOnlyList<ViewHandle> selected) =>
        _inner.CheckSelection(requestId, seat, selected);

    public BlockVerdictView CheckBlocks(int requestId, int seat, IReadOnlyList<BlockMessage> blocks) =>
        _inner.CheckBlocks(requestId, seat, blocks);

    public AttackVerdictView CheckAttacks(int requestId, int seat, IReadOnlyList<AttackMessage> attacks) =>
        _inner.CheckAttacks(requestId, seat, attacks);

    public bool CanConcede(int seat) => _inner.CanConcede(seat);

    public RulesStep Concede(int seat) => Observed(_inner.Concede(seat));

    public RulesStep Leave(int seat) => Observed(_inner.Leave(seat));

    public RulesStep? End(int? winner) => _inner.End(winner) is { } step ? Observed(step) : null;

    public PlayerView View(int seat) => _inner.View(seat);

    public ObjectView? Oracle(string cardId) => _inner.Oracle(cardId);

    public bool IsOver => _inner.IsOver;

    public GameOutcomeView Outcome => _inner.Outcome;

    public int WinningTeam => _inner.WinningTeam;

    public IReadOnlyList<RulesAnswer> History => _inner.History;

    public void Dispose()
    {
        _inner.Dispose();
        _observer.Closed(this);
    }

    private RulesStep Observed(RulesStep step, bool started = false)
    {
        _observer.Stepped(this, step, started);
        return step;
    }
}
