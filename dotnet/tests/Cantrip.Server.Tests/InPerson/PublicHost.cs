using Cantrip.Protocol;
using Cantrip.Server.Rules;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// A two-seat rules host that plays no rules: it opens with <see cref="Opening"/> and answers each answer with the next of
/// <see cref="Steps"/>, every step carrying a public projection as a host dealt with a public view does. For the relay's
/// tests, which are about what is sent where, not about any game.
/// <para>
/// Handles are kept apart by range so a test can tell whose space one is from: the public board's from 900, seat one's
/// from 100, seat two's from 200.
/// </para>
/// </summary>
internal sealed class PublicHost : IRulesHost
{
    public RulesStep Opening { get; set; } = Step([new RulesRequest(1, 1, new PriorityPromptMessage(), false, null)]);

    public Queue<RulesStep> Steps { get; } = new();

    public List<(int RequestId, int Seat)> Answers { get; } = [];

    public bool Disposed { get; private set; }

    public IReadOnlyList<int> Seats => [1, 2];

    public IReadOnlyList<string> SeatNames => ["Alice", "Bob"];

    public int Team(int seat) => seat;

    public IReadOnlyList<DeckCardView> Decklist(int seat) => [];

    public RulesStep Start() => Opening;

    public RulesStep Answer(int requestId, int seat, AnswerPayload answer)
    {
        Answers.Add((requestId, seat));
        return Steps.Dequeue();
    }

    public bool CanConcede(int seat) => true;

    public RulesStep Concede(int seat) => Step([], finished: true);

    public RulesStep Leave(int seat) => Step([], finished: true);

    public PlayerView View(int seat) => SeatView(seat);

    public ObjectView? Oracle(string cardId) => new() { CardId = cardId, Name = cardId };

    public bool IsOver { get; set; }

    public GameOutcomeView Outcome => IsOver ? GameOutcomeView.TeamWon : GameOutcomeView.InProgress;

    public int WinningTeam => IsOver ? 1 : 0;

    public IReadOnlyList<RulesAnswer> History => [];

    public void Dispose() => Disposed = true;

    /// <summary>
    /// A step with these requests outstanding: on the battlefield, a Mountain of seat one's (public 901, seat one's 101,
    /// seat two's 201) and a Tarmogoyf of seat two's (public 902, seat one's 102, seat two's 202), and in seat one's hand
    /// a Lightning Bolt only seat one sees (seat one's 103, no public handle).
    /// </summary>
    public static RulesStep Step(IReadOnlyList<RulesRequest> requests, bool finished = false, long sequence = 1,
        IReadOnlyList<(long Sequence, GameEventView View)>? events = null, Dictionary<int, Dictionary<ViewHandle, ViewHandle>>? handles = null)
    {
        handles ??= new()
        {
            [1] = new() { [new(901)] = new(101), [new(902)] = new(102) },
            [2] = new() { [new(901)] = new(201), [new(902)] = new(202) },
        };
        return new RulesStep(
            requests,
            new Dictionary<int, PlayerView> { [1] = SeatView(1), [2] = SeatView(2) },
            new Dictionary<int, IReadOnlyList<(long, GameEventView)>> { [1] = [], [2] = [] },
            sequence,
            finished,
            Public: new PublicProjection(
                PublicView(),
                events ?? [],
                handles.ToDictionary(each => each.Key, each => (IReadOnlyDictionary<ViewHandle, ViewHandle>)each.Value)));
    }

    public static PlayerView PublicView() => new()
    {
        Viewer = 0,
        Turn = 1,
        Objects =
        [
            new ObjectView { Handle = new(901), Zone = ZoneKindView.Battlefield, Owner = 1, CardId = "Mountain", Name = "Mountain" },
            new ObjectView { Handle = new(902), Zone = ZoneKindView.Battlefield, Owner = 2, CardId = "Tarmogoyf", Name = "Tarmogoyf" },
        ],
    };

    public static PlayerView SeatView(int seat) => new()
    {
        Viewer = seat,
        Turn = 1,
        Objects =
        [
            new ObjectView { Handle = new(seat * 100 + 1), Zone = ZoneKindView.Battlefield, Owner = 1, CardId = "Mountain", Name = "Mountain" },
            new ObjectView { Handle = new(seat * 100 + 2), Zone = ZoneKindView.Battlefield, Owner = 2, CardId = "Tarmogoyf", Name = "Tarmogoyf" },
            .. seat == 1 ? (ObjectView[])[new ObjectView { Handle = new(103), Zone = ZoneKindView.Hand, Owner = 1, CardId = "Lightning Bolt", Name = "Lightning Bolt" }] : [],
        ],
    };
}
