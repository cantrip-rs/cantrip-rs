using System.Reflection;
using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.Events;
using Cantrip.Server.InPerson;
using Cantrip.TestKit;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// The grid draft on the tablets (docs/in-person.md, "The grid draft"): what the relay shows the tablets of a real grid
/// draft - what both players are shown, nothing of either pool - and where a row or a column picked on a tablet goes: to
/// the phone of the seat whose pick it is, and nowhere else. No socket; the relay's clock is the test's.
/// </summary>
public sealed class InPersonGridTests : IDisposable
{
    private const string AliceTicket = "A11CE";
    private const string BobTicket = "B0B";

    private sealed class Clock : TimeProvider
    {
        private DateTimeOffset _now = new(2026, 10, 30, 12, 0, 0, TimeSpan.Zero);

        public override DateTimeOffset GetUtcNow() => _now;

        public void Advance(TimeSpan by) => _now += by;
    }

    private readonly string _data = Path.Combine(Path.GetTempPath(), "cantrip-in-person-grid-" + Guid.NewGuid().ToString("N"));
    private readonly Clock _clock = new();
    private readonly InPersonRelay _relay;
    private readonly DraftPod _pod;
    private readonly ObservedTableEvent _event;

    public InPersonGridTests()
    {
        var store = new EventStore(_data);
        CreatedEvent created = DraftPod.CreateGrid(store, Content, new GridDraftSetup
        {
            Name = "Grid at the table",
            Cube = "rw-aggro",
            CubeCards = CubeList.Load(Path.Combine(RepoRoot.Path, "decks", "rw-aggro.txt")),
            PickOrder = ["Lightning Bolt"],
            Packs = 2,
            MinimumDeck = 5,
            Seed = 11,
            Players = [("alice", "Alice"), ("bob", "Bob")],
        });
        _pod = DraftPod.Open(store, created.Record.Id, Content, _clock);
        _relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = ["Alice", "Bob"],
            SeatForTicket = ticket => ticket switch { AliceTicket => 1, BobTicket => 2, _ => 0 },
            TimeProvider = _clock,
        });
        _event = new ObservedTableEvent(_pod, _relay);
    }

    public void Dispose()
    {
        if (Directory.Exists(_data))
        {
            Directory.Delete(_data, recursive: true);
        }
    }

    private static Cantrip.Datasets.ContentLibrary Content => ContentCache.Get(RepoRoot.ContentRoot);

    private static List<DeviceServerMessage> Drain(RelayDevice device)
    {
        List<DeviceServerMessage> messages = [];
        while (device.Outbox.TryRead(out DeviceServerMessage? message)) messages.Add(message);
        return messages;
    }

    private RelayDevice Join(DeviceHelloMessage hello) => Joined(hello).Device;

    /// <summary>A device welcomed, and what it was sent after its welcome.</summary>
    private (RelayDevice Device, List<DeviceServerMessage> Sent) Joined(DeviceHelloMessage hello)
    {
        RelayDevice device = _relay.Attach();
        _relay.Receive(device, hello);
        List<DeviceServerMessage> sent = Drain(device);
        Assert.IsType<DeviceWelcomeMessage>(sent[0]);
        return (device, sent[1..]);
    }

    private RelayDevice Phone(string ticket) => Join(new DeviceHelloMessage { Ticket = ticket });

    private RelayDevice Tablet(int side = 0) => Join(new DeviceHelloMessage { Device = _relay.MintTablet(side) });

    /// <summary>Both seated and the draft begun, the table showing each seat the event as it does after every change.</summary>
    private DraftMessage Begin()
    {
        _event.Seated(1);
        _event.Seated(2);
        return Shown();
    }

    /// <summary>What the table shows seat 1 now - through the observed event, as the table asks it after every change.</summary>
    private DraftMessage Shown()
    {
        _event.ViewFor(2);
        return (DraftMessage)_event.ViewFor(1)!;
    }

    /// <summary>A pick made at the table, as the seat's phone sends it, and the table showing the seats the event again.</summary>
    private void Pick(int seat, int decision, int line)
    {
        Assert.Null(_event.Receive(seat, new DraftPickMessage { Decision = decision, Take = true, Card = line }));
        _event.ViewFor(1);
        _event.ViewFor(2);
    }

    private static int LineWithCards(DraftMessage draft) =>
        Enumerable.Range(0, 6).First(line => GridDraft.Places(line).Any(place => draft.Pile[place].Known));

    [Fact]
    public void The_tablets_are_shown_the_grid_as_both_players_see_it_and_nothing_of_either_pool()
    {
        RelayDevice tablet = Tablet();
        RelayDevice alice = Phone(AliceTicket);
        DraftMessage first = Begin();
        Pick(first.Active, first.Decision, LineWithCards(first));
        DraftMessage now = Shown();
        Drain(tablet);
        Drain(alice);

        // A late tablet is caught up with the grid as it stands, after where the event stands.
        (_, List<DeviceServerMessage> caughtUp) = Joined(new DeviceHelloMessage { Device = _relay.MintTablet(0) });
        Assert.IsType<DevicePhaseMessage>(caughtUp[0]);
        PublicGridMessage grid = Assert.IsType<PublicGridMessage>(Assert.Single(caughtUp.OfType<PublicGridMessage>()));

        Assert.Equal((now.Decision, now.PackNumber, now.Packs, now.PickNumber, now.Active), (grid.Decision, grid.Grid, grid.Grids, grid.Pick, grid.Active));
        Assert.Equal(now.Pile.Select(card => (card.Known, card.CardId, card.Name)), grid.Places.Select(card => (card.Known, card.CardId, card.Name)));
        Assert.Equal(now.Seats.OrderBy(seat => seat.Seat).Select(seat => seat.Drafted), grid.Drafted);
        Assert.Equal(now.LastLine, grid.LastLine);
        Assert.Equal(now.RecentCards.Single().Select(card => card.Name), grid.RecentCards.Single().Select(card => card.Name));

        // In words for nobody's seat: each player named, never "You".
        string said = Assert.Single(grid.Recent);
        Assert.StartsWith($"{(now.LastLine >= 0 ? _pod.NameAt(3 - now.Active) : string.Empty)} took", said, StringComparison.Ordinal);

        // Nothing either seat alone is shown: no pool, nor the cube's pick order each seat is told for its bots - the
        // message has no place for them - and no seat's routed pick ever reaches a tablet, nor the grid a phone.
        Assert.DoesNotContain(typeof(PublicGridMessage).GetProperties(), property => property.Name is "Pool" or "PileRanks" or "AdditionalPicks" or "PickDue");
        Assert.NotEmpty(now.PileRanks);
        Assert.All(Drain(tablet), message => Assert.IsNotType<SeatLineMessage>(message));
        Assert.DoesNotContain(Drain(alice), message => message is PublicGridMessage);
    }

    [Fact]
    public void A_line_picked_on_the_shared_tablet_goes_to_the_pickers_phone_alone_which_picks_it()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        DraftMessage draft = Begin();
        int picker = draft.Active;
        RelayDevice pickerPhone = picker == 1 ? alice : bob;
        RelayDevice otherPhone = picker == 1 ? bob : alice;
        Drain(alice);
        Drain(bob);
        Drain(tablet);
        int line = LineWithCards(draft);

        _relay.Receive(tablet, new TableLineMessage { Decision = draft.Decision, Line = line, Take = false });
        SeatLineMessage chosen = Assert.IsType<SeatLineMessage>(Assert.Single(Drain(pickerPhone)));
        Assert.Equal((draft.Decision, line, false), (chosen.Decision, chosen.Line, chosen.Take));

        _relay.Receive(tablet, new TableLineMessage { Decision = draft.Decision, Line = line, Take = true });
        SeatLineMessage taken = Assert.IsType<SeatLineMessage>(Assert.Single(Drain(pickerPhone)));
        Assert.True(taken.Take);
        Assert.Empty(Drain(otherPhone));

        // Nothing of the routing reaches a tablet: it learns of the pick only when the table makes it.
        Assert.Empty(Drain(tablet));
        Pick(picker, taken.Decision, taken.Line);
        PublicGridMessage after = Assert.IsType<PublicGridMessage>(Assert.Single(Drain(tablet)));
        Assert.Equal((draft.Decision + 1, 3 - picker, line), (after.Decision, after.Active, after.LastLine));
    }

    [Fact]
    public void A_line_for_a_pick_past_with_no_cards_or_out_of_the_grid_goes_nowhere()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();

        // No draft yet: nothing to pick.
        _relay.Receive(tablet, new TableLineMessage { Decision = 0, Line = 0, Take = true });
        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));

        DraftMessage first = Begin();
        int taken = LineWithCards(first);
        Pick(first.Active, first.Decision, taken);
        DraftMessage second = Shown();
        Drain(alice);
        Drain(bob);

        _relay.Receive(tablet, new TableLineMessage { Decision = first.Decision, Line = 3, Take = true });
        _relay.Receive(tablet, new TableLineMessage { Decision = second.Decision, Line = taken, Take = true });
        _relay.Receive(tablet, new TableLineMessage { Decision = second.Decision, Line = 6, Take = true });
        _relay.Receive(tablet, new TableLineMessage { Decision = second.Decision, Line = -1, Take = true });
        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));
    }

    [Fact]
    public void A_players_own_tablet_picks_only_on_that_players_pick()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice alices = Tablet(side: 1);
        RelayDevice bobs = Tablet(side: 2);
        DraftMessage draft = Begin();
        int picker = draft.Active;
        Drain(alice);
        Drain(bob);
        int line = LineWithCards(draft);

        _relay.Receive(picker == 1 ? bobs : alices, new TableLineMessage { Decision = draft.Decision, Line = line, Take = true });
        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));

        _relay.Receive(picker == 1 ? alices : bobs, new TableLineMessage { Decision = draft.Decision, Line = line, Take = true });
        Assert.Single(Drain(picker == 1 ? alice : bob));
        Assert.Empty(Drain(picker == 1 ? bob : alice));
    }

    [Fact]
    public void A_line_waits_for_no_phone_and_a_phone_cannot_pick_through_the_relay()
    {
        RelayDevice tablet = Tablet();
        DraftMessage draft = Begin();
        RelayDevice other = Phone(draft.Active == 1 ? BobTicket : AliceTicket);
        Drain(other);

        // The picker's phone is not at the table: the line goes nowhere, and nobody else is handed it.
        _relay.Receive(tablet, new TableLineMessage { Decision = draft.Decision, Line = LineWithCards(draft), Take = true });
        Assert.Empty(Drain(other));

        // A phone does not send lines; it is told so and keeps its place.
        _relay.Receive(other, new TableLineMessage { Decision = draft.Decision, Line = 0, Take = true });
        Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(other)));
        Assert.Equal(draft.Decision, ((DraftMessage)_pod.ViewFor(1)!).Decision);
    }

    [Fact]
    public void The_draft_over_puts_the_grid_away_and_a_clock_alone_is_not_news()
    {
        RelayDevice tablet = Tablet();
        DraftMessage first = Begin();
        Drain(tablet);

        // Asked again with nothing changed but time: nothing is sent.
        _clock.Advance(TimeSpan.FromSeconds(5));
        Shown();
        Assert.Empty(Drain(tablet));

        DraftMessage draft = first;
        while (draft.PackNumber > 0 && _pod.Phase == TablePhase.Drafting)
        {
            Pick(draft.Active, draft.Decision, LineWithCards(draft));
            if (_pod.Phase != TablePhase.Drafting) break;
            draft = Shown();
        }

        // Deck building: the event is shown to the seats as their decks, and the tablets are told the draft is over.
        _event.ViewFor(1);
        PublicGridMessage over = Drain(tablet).OfType<PublicGridMessage>().Last();
        Assert.Equal(0, over.Grid);
        Assert.Empty(over.Places);
        Assert.Equal(TablePhase.DeckBuilding, _pod.Phase);
    }

    /// <summary>
    /// Every member of <see cref="ITableEvent"/>, the ones with a default body too, is implemented by the wrapper itself:
    /// one left to its default would answer for the wrapper, not the event it wraps.
    /// </summary>
    [Fact]
    public void Every_member_of_the_table_event_is_forwarded()
    {
        InterfaceMapping map = typeof(ObservedTableEvent).GetInterfaceMap(typeof(ITableEvent));
        for (int i = 0; i < map.InterfaceMethods.Length; i++)
        {
            Assert.True(map.TargetMethods[i].DeclaringType == typeof(ObservedTableEvent),
                $"{map.InterfaceMethods[i].Name} is not forwarded by {nameof(ObservedTableEvent)}.");
        }

        Assert.Equal(_pod.EventId, _event.EventId);
        Assert.Equal(_pod.SeatFor("nothing"), _event.SeatFor("nothing"));
        Assert.Equal(_pod.NameAt(2), _event.NameAt(2));
        Assert.Equal(_pod.Name, _event.Name);
        Assert.Same(_pod, _event.Inner);
    }
}
