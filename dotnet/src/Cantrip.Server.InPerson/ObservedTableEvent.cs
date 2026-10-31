using Cantrip.Protocol;
using Cantrip.Server.Events;
using Cantrip.Server.Rules;

namespace Cantrip.Server.InPerson;

/// <summary>
/// An event that is another, forwarding every member of <see cref="ITableEvent"/> to it unchanged - the ones the interface
/// gives a default too, so the table sees exactly the event it would have had - and showing the relay the grid draft as
/// both players see it whenever the table shows a seat the event (<see cref="ITableEvent.ViewFor"/>): what the event
/// shows nobody's seat (<c>ViewFor(0)</c>), which is the draft with no pool. The seam the tablets' grid is fed through, as
/// <see cref="ObservedRulesHost"/> is the game's: on the table's own thread, with no change to the table. A test holds it
/// to forwarding every member by reflection.
/// </summary>
public sealed class ObservedTableEvent : ITableEvent
{
    private readonly ITableEvent _inner;
    private readonly InPersonRelay _relay;

    public ObservedTableEvent(ITableEvent inner, InPersonRelay relay)
    {
        _inner = inner ?? throw new ArgumentNullException(nameof(inner));
        _relay = relay ?? throw new ArgumentNullException(nameof(relay));
    }

    /// <summary>The event this one forwards to.</summary>
    public ITableEvent Inner => _inner;

    /// <summary>
    /// Shows the relay where the draft stands, as the public may see it; called on the table's thread whenever the table
    /// shows a seat the event, and once by the host before the table starts.
    /// </summary>
    public void Publish() =>
        _relay.Drafted(_inner.Phase == TablePhase.Drafting ? _inner.ViewFor(0) as DraftMessage : null);

    public string EventId => _inner.EventId;

    public string Name => _inner.Name;

    public int Seats => _inner.Seats;

    public TablePhase Phase => _inner.Phase;

    public string Detail => _inner.Detail;

    public int SeatFor(string ticket) => _inner.SeatFor(ticket);

    public string NameAt(int seat) => _inner.NameAt(seat);

    public string PlayerAt(int seat) => _inner.PlayerAt(seat);

    public void Seated(int seat) => _inner.Seated(seat);

    public ServerMessage? ViewFor(int seat)
    {
        ServerMessage? view = _inner.ViewFor(seat);
        Publish();
        return view;
    }

    public string? Receive(int seat, ClientMessage message) => _inner.Receive(seat, message);

    public bool ReadyToDeal => _inner.ReadyToDeal;

    public IRulesHost Deal(IReadOnlyList<string> names, IRulesHostFactory? rules = null) => _inner.Deal(names, rules);

    public void DealFailed(string why) => _inner.DealFailed(why);

    public void GameEnded(bool abandoned, IReadOnlyList<int> winningSeats, string detail) => _inner.GameEnded(abandoned, winningSeats, detail);

    public TimeSpan? ClockAtDeal(int seat) => _inner.ClockAtDeal(seat);

    public void ClocksKept(IReadOnlyDictionary<int, TimeSpan> left) => _inner.ClocksKept(left);

    public void ClockUsed(int seat, TimeSpan left) => _inner.ClockUsed(seat, left);

    public void ClockRanOut(int seat) => _inner.ClockRanOut(seat);

    public void ClocksRanOut(IReadOnlyList<int> seats) => _inner.ClocksRanOut(seats);

    public IReadOnlyList<int> TakeForfeits() => _inner.TakeForfeits();

    public string? ForfeitReason(int seat) => _inner.ForfeitReason(seat);

    public bool Forming => _inner.Forming;

    public void Connected(int seat, bool connected) => _inner.Connected(seat, connected);

    public bool Holds(int seat) => _inner.Holds(seat);

    public bool IsReady(int seat) => _inner.IsReady(seat);

    public TimeSpan? ReadyLeft(DateTimeOffset now) => _inner.ReadyLeft(now);

    public int Version => _inner.Version;

    public void Tick(DateTimeOffset now) => _inner.Tick(now);

    public IReadOnlyList<(int Seat, ServerMessage Farewell)> TakeVacated() => _inner.TakeVacated();
}
