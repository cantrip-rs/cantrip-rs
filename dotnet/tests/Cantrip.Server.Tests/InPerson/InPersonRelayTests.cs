using Cantrip.Protocol;
using Cantrip.Protocol.InPerson;
using Cantrip.Server.InPerson;
using Cantrip.Server.Rules;
using MessagePack;
using Xunit;

namespace Cantrip.Server.Tests.InPerson;

/// <summary>
/// In-person play's relay (docs/in-person.md), with no socket and a clock the test moves: what each device is sent, how a
/// tablet's tap reaches a seat's phone, and who may take which place at the table.
/// </summary>
public sealed class InPersonRelayTests
{
    private const string AliceTicket = "A11CE";
    private const string BobTicket = "B0B";

    private sealed class Clock : TimeProvider
    {
        private DateTimeOffset _now = new(2026, 10, 30, 12, 0, 0, TimeSpan.Zero);

        public override DateTimeOffset GetUtcNow() => _now;

        public void Advance(TimeSpan by) => _now += by;
    }

    private readonly Clock _clock = new();
    private readonly InPersonRelay _relay;

    public InPersonRelayTests()
    {
        _relay = new InPersonRelay(new InPersonRelayOptions
        {
            PlayerNames = ["Alice", "Bob"],
            SeatForTicket = ticket => ticket switch { AliceTicket => 1, BobTicket => 2, _ => 0 },
            TimeProvider = _clock,
        });
    }

    private static List<DeviceServerMessage> Drain(RelayDevice device)
    {
        List<DeviceServerMessage> messages = [];
        while (device.Outbox.TryRead(out DeviceServerMessage? message)) messages.Add(message);
        return messages;
    }

    private RelayDevice Join(DeviceHelloMessage hello)
    {
        RelayDevice device = _relay.Attach();
        _relay.Receive(device, hello);
        return device;
    }

    /// <summary>A phone seated by its ticket, its welcome already read.</summary>
    private RelayDevice Phone(string ticket)
    {
        RelayDevice phone = Join(new DeviceHelloMessage { Ticket = ticket });
        Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(phone)));
        return phone;
    }

    private RelayDevice Tablet(int side = 0) => Join(new DeviceHelloMessage { Device = _relay.MintTablet(side) });

    /// <summary>A game dealt through the relay's factory, started: seat one to act, with request 1.</summary>
    private (ObservedRulesHost Game, PublicHost Inner) Deal(RulesStep? opening = null)
    {
        var inner = new PublicHost();
        if (opening is not null) inner.Opening = opening;
        var game = (ObservedRulesHost)new ObservedRulesFactory(new Fixed(inner), _relay).Create([], 1, new RulesGameOptions { MinimumDeckSize = 40 });
        game.Start();
        return (game, inner);
    }

    private sealed class Fixed(IRulesHost host) : IRulesHostFactory
    {
        public IRulesHost Create(IReadOnlyList<RulesSeat> seats, ulong seed, RulesGameOptions options) => host;
    }

    private static RulesRequest Priority(int requestId, int seat) => new(requestId, seat, new PriorityPromptMessage(), false, null);

    [Fact]
    public void A_tap_on_a_permanent_goes_to_the_acting_seat_in_its_own_handles()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(5, 1)]));
        Drain(alice);
        Drain(bob);

        _relay.Receive(tablet, new TableTapMessage { Sequence = 1, Object = new(902) });

        SeatTapMessage tap = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(alice)));
        Assert.Equal(5, tap.RequestId);
        Assert.Equal(new ViewHandle(102), tap.Object);
        Assert.Equal(0, tap.Player);
        Assert.Empty(Drain(bob));
    }

    [Fact]
    public void A_tap_the_phone_did_not_use_goes_to_the_next_request()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();

        // Both seats asked at once, as when each declares blockers: seat two's request is the earlier.
        Deal(PublicHost.Step([Priority(9, 1), Priority(4, 2)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });

        SeatTapMessage first = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(bob)));
        Assert.Equal((4, new ViewHandle(201)), (first.RequestId, first.Object));
        Assert.Empty(Drain(alice));

        _relay.Receive(bob, new TapResultMessage { TapId = first.TapId, Used = false });
        SeatTapMessage second = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(alice)));
        Assert.Equal((9, new ViewHandle(101)), (second.RequestId, second.Object));

        // Not used there either, and nothing is left to ask: the tap goes nowhere.
        _relay.Receive(alice, new TapResultMessage { TapId = second.TapId, Used = false });
        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));
    }

    [Fact]
    public void A_tap_used_goes_no_further()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1), Priority(2, 2)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(902) });
        SeatTapMessage tap = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(alice)));

        _relay.Receive(alice, new TapResultMessage { TapId = tap.TapId, Used = true });
        _clock.Advance(TimeSpan.FromMinutes(1));
        _relay.Tick();

        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));
    }

    [Fact]
    public void A_phone_that_says_nothing_of_a_tap_lets_it_go_on_once_its_time_is_up()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1), Priority(2, 2)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(902) });
        Assert.Single(Drain(alice));

        _clock.Advance(TimeSpan.FromSeconds(2));
        _relay.Tick();
        Assert.Empty(Drain(bob));

        _clock.Advance(TimeSpan.FromSeconds(2));
        _relay.Tick();
        SeatTapMessage onward = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(bob)));
        Assert.Equal((2, new ViewHandle(202)), (onward.RequestId, onward.Object));
    }

    [Fact]
    public void A_tap_with_no_request_outstanding_does_nothing()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });
        _relay.Receive(tablet, new TableTapMessage { Player = 2 });
        Assert.Empty(Drain(alice));
        Assert.Empty(Drain(bob));
    }

    [Fact]
    public void A_tap_before_any_game_does_nothing()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice tablet = Tablet();
        Drain(alice);
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });
        Assert.Empty(Drain(alice));
    }

    [Fact]
    public void An_object_a_seat_does_not_name_is_never_sent_to_it()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1), Priority(2, 2)], handles: new()
        {
            [1] = new() { [new(901)] = new(101) },
            [2] = new() { [new(901)] = new(201), [new(902)] = new(202) },
        }));

        _relay.Receive(tablet, new TableTapMessage { Object = new(902) });

        Assert.Empty(Drain(alice));
        SeatTapMessage tap = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(bob)));
        Assert.Equal(new ViewHandle(202), tap.Object);
    }

    [Fact]
    public void A_tap_on_a_player_goes_by_seat()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice tablet = Tablet(2);
        Deal(PublicHost.Step([Priority(3, 1)]));
        _relay.Receive(tablet, new TableTapMessage { Player = 2 });

        SeatTapMessage tap = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(alice)));
        Assert.Equal((3, ViewHandle.None, 2), (tap.RequestId, tap.Object, tap.Player));

        // No such seat, or a tap naming both an object and a player: nothing.
        _relay.Receive(tablet, new TableTapMessage { Player = 3 });
        _relay.Receive(tablet, new TableTapMessage { Object = new(901), Player = 1 });
        Assert.Empty(Drain(alice));
    }

    [Fact]
    public void A_seat_with_no_phone_connected_is_passed_over()
    {
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1), Priority(2, 2)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });
        Assert.Equal(2, Assert.IsType<SeatTapMessage>(Assert.Single(Drain(bob))).RequestId);
    }

    [Fact]
    public void A_phone_answers_only_for_its_own_taps()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice bob = Phone(BobTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1), Priority(2, 2)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });
        SeatTapMessage tap = Assert.IsType<SeatTapMessage>(Assert.Single(Drain(alice)));

        // Bob cannot send Alice's tap on, nor is a tablet heard saying what a tap was.
        _relay.Receive(bob, new TapResultMessage { TapId = tap.TapId, Used = false });
        Assert.Empty(Drain(bob));
        _relay.Receive(tablet, new TapResultMessage { TapId = tap.TapId, Used = false });
        Assert.Empty(Drain(bob));
        Assert.Single(Drain(tablet).OfType<DeviceErrorMessage>());
    }

    [Fact]
    public void A_tablet_is_sent_the_public_feed_and_nothing_of_any_prompt()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice tablet = Tablet();
        (ObservedRulesHost game, PublicHost inner) = Deal(PublicHost.Step([Priority(1, 1)]));
        inner.Steps.Enqueue(PublicHost.Step([Priority(2, 2)], sequence: 2, events: [(2, new StartingPlayerChosenEvent { Chooser = 1 })]));
        inner.Steps.Enqueue(PublicHost.Step([], finished: true, sequence: 3));
        game.Answer(1, 1, new PassAnswer());
        inner.IsOver = true;
        game.Answer(2, 2, new PassAnswer());
        _relay.Receive(tablet, new TableTapMessage { Object = new(902) });
        _relay.Receive(alice, new TapResultMessage { TapId = 1, Used = false });

        List<DeviceServerMessage> sent = Drain(tablet);
        Assert.IsType<DeviceWelcomeMessage>(sent[0]);
        List<ServerMessage> feed = [.. sent.OfType<PublicFeedMessage>().Select(each => each.Message)];
        Assert.All(sent, message => Assert.True(message is DeviceWelcomeMessage or PublicFeedMessage or DevicePhaseMessage, message.Describe()));
        Assert.All(feed, message => Assert.True(message is WelcomeMessage { Seat: 0 } or StateMessage or EventsMessage or GameOverMessage, message.Describe()));
        Assert.Equal(3, feed.OfType<StateMessage>().Count());
        Assert.Single(feed.OfType<GameOverMessage>());
        Assert.Contains(feed.OfType<EventsMessage>(), events => events.Events.Count == 1 && events.Sequences.SequenceEqual([2L]));

        // Every handle is the public board's own; no seat's, and nothing of the hand only seat one sees.
        Assert.All(feed.OfType<StateMessage>().SelectMany(state => state.View.Objects), each => Assert.InRange(each.Handle.Value, 900, 999));
        Assert.DoesNotContain(feed.OfType<StateMessage>().SelectMany(state => state.Printed), card => card.CardId == "Lightning Bolt");
        Assert.Equal(["Mountain", "Tarmogoyf"], feed.OfType<StateMessage>().SelectMany(state => state.Printed).Select(card => card.CardId));
        Assert.All(feed.OfType<StateMessage>(), state => Assert.All(state.View.Objects, each => Assert.NotEqual(0, each.Fingerprint)));
    }

    [Fact]
    public void A_tablet_joining_late_is_brought_up_to_date_with_every_card_so_far()
    {
        (ObservedRulesHost game, PublicHost inner) = Deal(PublicHost.Step([Priority(1, 1)], sequence: 4));
        RelayDevice tablet = Tablet();
        List<ServerMessage> feed = [.. Drain(tablet).OfType<PublicFeedMessage>().Select(each => each.Message)];

        Assert.IsType<WelcomeMessage>(feed[0]);
        StateMessage state = Assert.IsType<StateMessage>(feed[1]);
        Assert.Equal(["Mountain", "Tarmogoyf"], state.Printed.Select(card => card.CardId));
        Assert.Equal(4, Assert.IsType<EventsMessage>(feed[2]).Sequence);
        Assert.Equal(3, feed.Count);
        _ = game;
        _ = inner;
    }

    [Fact]
    public void Before_the_game_a_tablet_is_told_where_the_event_stands_and_who_is_here()
    {
        RelayDevice tablet = Tablet();
        DevicePhaseMessage first = Assert.IsType<DevicePhaseMessage>(Drain(tablet)[1]);
        Assert.Equal(["The shared tablet"], first.Connected);

        _relay.Phase(TablePhase.Drafting, "Grid 1 of 18: Alice picks.");
        DevicePhaseMessage drafting = Assert.IsType<DevicePhaseMessage>(Assert.Single(Drain(tablet)));
        Assert.Equal((TablePhase.Drafting, "Grid 1 of 18: Alice picks."), (drafting.Phase, drafting.Line));

        // The same again says nothing; a phone arriving does.
        _relay.Phase(TablePhase.Drafting, "Grid 1 of 18: Alice picks.");
        Assert.Empty(Drain(tablet));
        Phone(AliceTicket);
        Assert.Equal(["Alice's phone", "The shared tablet"], Assert.IsType<DevicePhaseMessage>(Assert.Single(Drain(tablet))).Connected);
    }

    [Fact]
    public void A_phone_is_never_sent_the_public_feed()
    {
        RelayDevice alice = Phone(AliceTicket);
        Deal(PublicHost.Step([Priority(1, 1)]));
        _relay.Phase(TablePhase.Playing, "Game 1.");
        Assert.Empty(Drain(alice));
    }

    [Fact]
    public void A_ticket_seats_its_phone_and_a_token_its_tablet()
    {
        RelayDevice alice = Join(new DeviceHelloMessage { Ticket = AliceTicket });
        DeviceWelcomeMessage welcome = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(alice)));
        Assert.Equal((DeviceRole.Phone, 1, 0), (welcome.Role, welcome.Seat, welcome.Side));
        Assert.Equal(["Alice", "Bob"], welcome.PlayerNames);
        Assert.Equal(32, welcome.Proof.Length);

        RelayDevice tablet = Tablet(2);
        DeviceWelcomeMessage tabletWelcome = Assert.IsType<DeviceWelcomeMessage>(Drain(tablet)[0]);
        Assert.Equal((DeviceRole.Tablet, 0, 2), (tabletWelcome.Role, tabletWelcome.Seat, tabletWelcome.Side));

        Assert.Equal([("Alice's phone", true), ("Bob's phone", false), ("Bob's tablet", true)],
            _relay.Devices.Select(device => (device.Name, device.Connected)));
    }

    [Fact]
    public void A_wrong_code_is_refused_and_the_connection_ended()
    {
        _relay.MintTablet(0);
        foreach (DeviceHelloMessage hello in (DeviceHelloMessage[])[
            new() { Ticket = "nope" },
            new() { Device = "nope" },
            new() { ResumeId = "nope", Proof = new byte[32] },
            new(),
        ])
        {
            RelayDevice device = Join(hello);
            DeviceErrorMessage refused = Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(device)));
            Assert.Equal(ErrorCode.MembershipRefused, refused.Code);
            Assert.True(device.Outbox.Completion.IsCompleted);
        }

        Assert.All(_relay.Devices, device => Assert.False(device.Connected));
    }

    [Fact]
    public void A_device_says_who_it_is_first_and_in_the_versions_the_relay_speaks()
    {
        RelayDevice early = _relay.Attach();
        _relay.Receive(early, new TableTapMessage { Object = new(901) });
        Assert.Equal(ErrorCode.MembershipRequired, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(early))).Code);

        RelayDevice old = Join(new DeviceHelloMessage { Ticket = AliceTicket, InPersonVersion = InPersonProtocol.Version - 1 });
        Assert.Equal(ErrorCode.ProtocolMismatch, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(old))).Code);
        Assert.False(_relay.Devices[0].Connected);
    }

    [Fact]
    public void A_device_resumes_its_place_with_its_proof_and_not_without_it()
    {
        RelayDevice first = Join(new DeviceHelloMessage { Ticket = AliceTicket });
        DeviceWelcomeMessage welcome = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(first)));
        _relay.Detach(first);

        byte[] wrong = [.. welcome.Proof];
        wrong[0] ^= 1;
        RelayDevice forged = Join(new DeviceHelloMessage { ResumeId = welcome.ResumeId, Proof = wrong });
        Assert.Equal(ErrorCode.MembershipRefused, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(forged))).Code);

        RelayDevice back = Join(new DeviceHelloMessage { ResumeId = welcome.ResumeId, Proof = welcome.Proof });
        DeviceWelcomeMessage again = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(back)));
        Assert.Equal((1, welcome.ResumeId), (again.Seat, again.ResumeId));
        Assert.Equal(welcome.Proof, again.Proof);
        Assert.True(_relay.Devices[0].Connected);
    }

    [Fact]
    public void Scanning_the_code_again_takes_the_place_over_and_the_old_proof_no_longer_works()
    {
        RelayDevice first = Join(new DeviceHelloMessage { Ticket = AliceTicket });
        DeviceWelcomeMessage welcome = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(first)));

        RelayDevice second = Join(new DeviceHelloMessage { Ticket = AliceTicket });
        DeviceWelcomeMessage taken = Assert.IsType<DeviceWelcomeMessage>(Assert.Single(Drain(second)));
        Assert.Equal(ErrorCode.MembershipRefused, Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(first))).Code);
        Assert.True(first.Outbox.Completion.IsCompleted);
        Assert.NotEqual(welcome.Proof, taken.Proof);

        // The first device's going later leaves the second in its place.
        _relay.Detach(first);
        Assert.True(_relay.Devices[0].Connected);

        RelayDevice stale = Join(new DeviceHelloMessage { ResumeId = welcome.ResumeId, Proof = welcome.Proof });
        Assert.IsType<DeviceErrorMessage>(Assert.Single(Drain(stale)));

        // A tap goes to the device that holds the seat now.
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1)]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });
        Assert.IsType<SeatTapMessage>(Assert.Single(Drain(second)));
    }

    [Fact]
    public void A_new_game_starts_the_public_board_afresh()
    {
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1)]));
        Drain(tablet);

        (ObservedRulesHost next, _) = Deal(PublicHost.Step([Priority(1, 1)]));
        List<ServerMessage> feed = [.. Drain(tablet).OfType<PublicFeedMessage>().Select(each => each.Message)];
        Assert.IsType<WelcomeMessage>(feed[0]);
        Assert.Equal(["Mountain", "Tarmogoyf"], Assert.IsType<StateMessage>(feed[1]).Printed.Select(card => card.CardId));
        _ = next;
    }

    [Fact]
    public void Steps_of_a_game_no_longer_played_are_not_shown()
    {
        (ObservedRulesHost old, PublicHost oldInner) = Deal(PublicHost.Step([Priority(1, 1)]));
        Deal(PublicHost.Step([Priority(1, 1)]));
        RelayDevice tablet = Tablet();
        Drain(tablet);

        oldInner.Steps.Enqueue(PublicHost.Step([], finished: true));
        old.Answer(1, 1, new PassAnswer());
        Assert.Empty(Drain(tablet));
    }

    [Fact]
    public void Every_message_a_device_is_sent_goes_over_the_wire()
    {
        RelayDevice alice = Phone(AliceTicket);
        RelayDevice tablet = Tablet();
        Deal(PublicHost.Step([Priority(1, 1)], events: [(1, new StartingPlayerChosenEvent { Chooser = 1 })]));
        _relay.Receive(tablet, new TableTapMessage { Object = new(901) });

        foreach (DeviceServerMessage message in Drain(alice).Concat(Drain(tablet)))
        {
            byte[] bytes = MessagePackSerializer.Serialize(message);
            Assert.Equal(message.GetType(), MessagePackSerializer.Deserialize<DeviceServerMessage>(bytes).GetType());
        }
    }
}
