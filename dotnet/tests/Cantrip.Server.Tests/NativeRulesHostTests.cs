using System.Text.Json;
using Cantrip.Core;
using Cantrip.Protocol;
using Cantrip.Server.Rules;
using Xunit;

namespace Cantrip.Server.Tests;

/// <summary>
/// <see cref="NativeRulesHost"/> against a host that answers from a script rather than the real sidecar: that it sends
/// the requests <c>docs/host-protocol.md</c> lays out, byte for byte, and that what comes back reaches a
/// <see cref="GameSession"/>'s seats as messages.
/// </summary>
public sealed class NativeRulesHostTests
{
    private static readonly PlayerId One = new(1);
    private static readonly PlayerId Two = new(2);

    private static readonly RulesSeat[] Seats =
    [
        new("Alice", ["Island", "Island", "Lightning Bolt"], ["Counterspell"]),
        new("Bob", ["Swamp"]),
    ];

    [Fact]
    public void A_deal_asks_the_host_for_a_new_game_and_lists_each_deck_for_the_welcome()
    {
        var host = new ScriptedHost();
        NativeRulesHost rules = NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 });

        Assert.Equal(
            """{"Cmd":"NewGame","Seats":[{"Name":"Alice","Deck":["Island","Island","Lightning Bolt"],"Sideboard":["Counterspell"],"Companion":null},{"Name":"Bob","Deck":["Swamp"],"Sideboard":[],"Companion":null}],"Seed":123,"Shuffle":true,"FirstPlayer":null,"Chooser":null,"BasicLandsOutside":[],"MinimumDeckSize":60}""",
            Assert.Single(host.Requests));
        Assert.Equal(1, rules.GameId);
        Assert.Equal([1, 2], rules.Seats);
        Assert.Equal(["Alice", "Bob"], rules.SeatNames);

        // By name with a count, in name order: the host sends the deck card by card, which a decklist must not.
        Assert.Equal([("Island", 2), ("Lightning Bolt", 1)], rules.Decklist(1).Select(entry => (entry.Name, entry.Count)));
        Assert.Equal("Island", rules.Decklist(1)[0].Card.CardId);
        Assert.Equal([("Swamp", 1)], rules.Decklist(2).Select(entry => (entry.Name, entry.Count)));
    }

    [Fact]
    public void A_seats_deck_name_is_told_to_the_host_when_it_has_one()
    {
        var host = new ScriptedHost();
        RulesSeat[] seats = [new("Alice", ["Island"]) { DeckName = "ub-reanimator" }, new("Bob", ["Swamp"])];
        NativeRulesHost.NewGame(host, seats, 5, new RulesGameOptions { MinimumDeckSize = 60 });

        Assert.Equal(
            """{"Cmd":"NewGame","Seats":[{"Name":"Alice","Deck":["Island"],"Sideboard":[],"Companion":null,"DeckName":"ub-reanimator"},{"Name":"Bob","Deck":["Swamp"],"Sideboard":[],"Companion":null}],"Seed":5,"Shuffle":true,"FirstPlayer":null,"Chooser":null,"BasicLandsOutside":[],"MinimumDeckSize":60}""",
            Assert.Single(host.Requests));
    }

    [Fact]
    public void A_refused_deal_is_a_rejection_with_the_hosts_code()
    {
        var host = new ScriptedHost { RefuseDeal = ("InvalidSetup", "No such card: Wish.") };

        RulesRejectedException refused = Assert.Throws<RulesRejectedException>(
            () => NativeRulesHost.NewGame(host, Seats, 1, new RulesGameOptions { MinimumDeckSize = 60 }));

        Assert.Equal(ErrorCode.InvalidSetup, refused.Code);
        Assert.Equal("No such card: Wish.", refused.Detail);
    }

    [Fact]
    public void Starting_publishes_the_opening_through_the_session_as_the_engine_would()
    {
        var host = new ScriptedHost();
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();

        Assert.Equal("""{"Cmd":"Start","GameId":1}""", host.Requests[1]);

        List<ServerMessage> one = [.. session.Drain(One)];
        WelcomeMessage welcome = Assert.IsType<WelcomeMessage>(one[0]);
        Assert.Equal(1, welcome.Seat);
        Assert.Equal(["Alice", "Bob"], welcome.PlayerNames);
        Assert.Equal(["Island", "Lightning Bolt"], welcome.Decklist.Select(entry => entry.Name));

        // The view, with nothing printed again: the only card in it is in the deck the welcome carried.
        StateMessage state = Assert.IsType<StateMessage>(one[1]);
        Assert.Equal(1, state.View.Viewer);
        Assert.Equal("Lightning Bolt", Assert.Single(state.View.Objects).CardId);
        Assert.NotEqual(0, state.View.Objects[0].Fingerprint);
        Assert.Empty(state.Printed);

        // The host numbers no events, so they are counted here: seat one's, then seat two's, and the log stands at two.
        EventsMessage events = Assert.IsType<EventsMessage>(one[2]);
        Assert.IsType<StepBeganEvent>(Assert.Single(events.Events));
        Assert.Equal([1], events.Sequences);
        Assert.Equal(2, events.Sequence);

        // A step beginning is a heading, not a line, so there is no log message for it.
        PromptMessage prompt = Assert.IsType<PromptMessage>(one[3]);
        Assert.Equal(7, prompt.RequestId);
        Assert.IsType<PriorityPromptMessage>(prompt.Prompt);
        Assert.False(prompt.MayCancel);
        Assert.Equal(4, one.Count);

        // Seat two sees an Island that is not in its own deck, so it is sent as printed, asked of the host once.
        List<ServerMessage> two = [.. session.Drain(Two)];
        StateMessage theirs = Assert.Single(two.OfType<StateMessage>());
        Assert.Equal("Island", Assert.Single(theirs.Printed).CardId);
        Assert.Equal("""{"Cmd":"Oracle","Name":"Island"}""", Assert.Single(host.Requests, request => request.Contains("Oracle", StringComparison.Ordinal)));
        Assert.Empty(two.OfType<PromptMessage>());
        Assert.Equal([1], session.WaitingOn);
    }

    [Fact]
    public void An_answer_is_sent_as_the_doc_writes_it_and_the_next_step_published()
    {
        var host = new ScriptedHost();
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();
        session.Drain(One);
        session.Drain(Two);

        session.Submit(One, new AnswerMessage { RequestId = 7, Answer = new PassAnswer() });

        Assert.Equal("""{"Cmd":"Answer","GameId":1,"RequestId":7,"Seat":1,"Answer":{"$type":"PassAnswer"}}""", host.Requests[^1]);
        Assert.Empty(session.Drain(One).OfType<ErrorMessage>());
        PromptMessage next = Assert.Single(session.Drain(Two).OfType<PromptMessage>());
        Assert.Equal(8, next.RequestId);
        Assert.Equal([2], session.WaitingOn);
        Assert.Equal([(1, typeof(PassAnswer))], session.Rules.History.Select(answer => (answer.Seat, answer.Answer.GetType())));
    }

    [Fact]
    public void A_rejection_is_an_error_and_the_same_question_again()
    {
        var host = new ScriptedHost();
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();
        session.Drain(One);

        session.Submit(One, new AnswerMessage { RequestId = 7, Answer = new CardsChosenAnswer() });

        List<ServerMessage> sent = [.. session.Drain(One)];
        Assert.Equal(ErrorCode.IllegalAnswer, Assert.Single(sent.OfType<ErrorMessage>()).Code);
        Assert.Equal(7, Assert.Single(sent.OfType<PromptMessage>()).RequestId);
        Assert.Empty(session.Rules.History);

        // A stale reply names a question that is no longer there, so there is nothing to ask again.
        host.Rejections["StaleReply"] = true;
        session.Submit(One, new AnswerMessage { RequestId = 7, Answer = new PassAnswer() });
        sent = [.. session.Drain(One)];
        Assert.Equal(ErrorCode.StaleReply, Assert.Single(sent.OfType<ErrorMessage>()).Code);
        Assert.Empty(sent.OfType<PromptMessage>());
    }

    [Fact]
    public void Conceding_is_offered_only_when_it_ends_the_game_and_ends_it()
    {
        var host = new ScriptedHost();
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));

        Assert.False(session.CanConcede(Two));
        session.Start();
        Assert.True(session.CanConcede(Two));

        session.Concede(Two);

        Assert.Equal("""{"Cmd":"Concede","GameId":1,"Seat":2}""", host.Requests[^1]);
        Assert.True(session.IsOver);
        Assert.Equal([1], session.WinningSeats);
        foreach (PlayerId seat in (PlayerId[])[One, Two])
        {
            GameOverMessage over = Assert.Single(session.Drain(seat).OfType<GameOverMessage>());
            Assert.Equal((GameOutcomeView.TeamWon, 1), (over.Outcome, over.WinningTeam));
        }

        Assert.False(session.CanConcede(One));
    }

    [Fact]
    public void Resynchronising_asks_the_host_for_the_seats_view_and_sends_everything_printed_so_far()
    {
        var host = new ScriptedHost();
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();
        session.Drain(Two);

        session.Resynchronize(Two);

        Assert.Equal("""{"Cmd":"View","GameId":1,"Seat":2}""", host.Requests[^1]);
        List<ServerMessage> sent = [.. session.Drain(Two)];
        Assert.IsType<WelcomeMessage>(sent[0]);
        StateMessage state = Assert.IsType<StateMessage>(sent[1]);
        Assert.Equal(2, state.View.Viewer);
        Assert.Equal(["Swamp", "Island"], state.Printed.Select(card => card.CardId));
        Assert.Equal(2, Assert.IsType<EventsMessage>(sent[2]).Sequence);
        Assert.True(Assert.IsType<LogMessage>(sent[3]).Complete);
    }

    [Fact]
    public void A_card_is_asked_for_as_printed_once()
    {
        var host = new ScriptedHost();
        NativeRulesHost rules = NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 });

        Assert.Equal("Island", rules.Oracle("Island")!.Name);
        Assert.Equal("Island", rules.Oracle("Island")!.Name);
        Assert.Null(rules.Oracle("Wish"));
        Assert.Null(rules.Oracle("Wish"));

        Assert.Equal(2, host.Requests.Count(request => request.Contains("\"Oracle\"", StringComparison.Ordinal)));
    }

    [Fact]
    public void Disposing_closes_the_game_in_the_host()
    {
        var host = new ScriptedHost();
        var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));

        session.Dispose();
        session.Dispose();

        Assert.Equal("""{"Cmd":"Close","GameId":1}""", Assert.Single(host.Requests, request => request.Contains("Close", StringComparison.Ordinal)));
    }

    [Fact]
    public void A_resumed_game_is_dealt_again_and_given_its_answers_again()
    {
        var host = new ScriptedHost();
        NativeRulesHost rules = NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }, history: [new RulesAnswer(1, new PassAnswer())]);
        using var session = new GameSession(rules);

        session.Start();

        Assert.Equal(
            ["""{"Cmd":"Start","GameId":1}""", """{"Cmd":"Answer","GameId":1,"RequestId":7,"Seat":1,"Answer":{"$type":"PassAnswer"}}"""],
            host.Requests.Skip(1).Take(2));

        // The position is the one reached, and the log has everything that happened on the way.
        Assert.Equal(8, Assert.Single(session.Drain(Two).OfType<PromptMessage>()).RequestId);
        List<ServerMessage> one = [.. session.Drain(One)];
        Assert.Empty(one.OfType<PromptMessage>());
        Assert.Equal([1, 3], Assert.Single(one.OfType<EventsMessage>()).Sequences);
        Assert.Single(rules.History);
    }

    [Fact]
    public void A_history_that_does_not_replay_fails_loudly()
    {
        var host = new ScriptedHost();
        NativeRulesHost rules = NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }, history: [new RulesAnswer(1, new CardsChosenAnswer())]);

        Assert.Throws<InvalidOperationException>(rules.Start);
    }

    [Fact]
    public void Events_the_host_numbers_keep_their_numbers()
    {
        var host = new ScriptedHost { Numbered = true };
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();

        EventsMessage events = Assert.Single(session.Drain(One).OfType<EventsMessage>());
        Assert.Equal([40], events.Sequences);
        Assert.Equal(40, events.Sequence);
    }

    [Fact]
    public void A_card_choice_the_host_sends_bare_gets_its_wording_facts_from_the_view()
    {
        var host = new ScriptedHost { Opening = ScriptedHost.DiscardStep };
        using var session = new GameSession(NativeRulesHost.NewGame(host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }));
        session.Start();

        PromptMessage prompt = Assert.Single(session.Drain(One).OfType<PromptMessage>());
        PromptPresentationContext facts = Assert.IsType<PromptPresentationContext>(prompt.Presentation);
        Assert.Equal((PromptActionView.Discard, PromptCauseView.Effect, 1, 1), (facts.Action, facts.Cause, facts.Chooser, facts.Owner));
        Assert.Equal((1, 1), (facts.Minimum, facts.Maximum));
        PromptOriginView origin = Assert.Single(facts.Origins);
        Assert.Equal((ZoneKindView.Hand, 1), (origin.Zone, origin.Owner));
    }

    [Fact]
    public void The_factory_deals_on_the_transport_it_is_given()
    {
        var host = new ScriptedHost();
        IRulesHost rules = new NativeRulesHostFactory(host).Create(Seats, 9, new RulesGameOptions { MinimumDeckSize = 60, FirstPlayer = 2, Shuffle = false });

        Assert.IsType<NativeRulesHost>(rules);
        Assert.Contains("\"Seed\":9,\"Shuffle\":false,\"FirstPlayer\":2", Assert.Single(host.Requests), StringComparison.Ordinal);
        Assert.DoesNotContain("Public", host.Requests[0], StringComparison.Ordinal);
        Assert.Null(rules.Start().Public);
    }

    [Fact]
    public void A_factory_with_a_public_view_tells_the_host_so_as_it_deals()
    {
        var host = new ScriptedHost();
        new NativeRulesHostFactory(host) { PublicView = true }.Create(Seats, 9, new RulesGameOptions { MinimumDeckSize = 60 });

        Assert.Equal(
            """{"Cmd":"NewGame","Seats":[{"Name":"Alice","Deck":["Island","Island","Lightning Bolt"],"Sideboard":["Counterspell"],"Companion":null},{"Name":"Bob","Deck":["Swamp"],"Sideboard":[],"Companion":null}],"Seed":9,"Shuffle":true,"FirstPlayer":null,"Chooser":null,"BasicLandsOutside":[],"MinimumDeckSize":60,"Public":true}""",
            Assert.Single(host.Requests));
    }

    [Fact]
    public void A_steps_public_part_is_read_with_its_view_stamped_its_events_numbered_and_each_seats_handle_map()
    {
        var host = new ScriptedHost { Numbered = true, Public = true };
        RulesStep step = NativeRulesHost.NewGame(host, Seats, 1, new RulesGameOptions { MinimumDeckSize = 60 }, publicView: true).Start();

        PublicProjection shown = Assert.IsType<PublicProjection>(step.Public);
        Assert.Equal(0, shown.View.Viewer);
        ObjectView island = Assert.Single(shown.View.Objects);
        Assert.Equal((new ViewHandle(5), "Island"), (island.Handle, island.Name));
        Assert.Equal(ObjectFingerprint.Of(island), island.Fingerprint);
        Assert.NotEqual(0, island.Fingerprint);
        (long sequence, GameEventView began) = Assert.Single(shown.Events);
        Assert.Equal(40, sequence);
        Assert.IsType<StepBeganEvent>(began);
        Assert.Empty(shown.Handles[1]);
        Assert.Equal(new ViewHandle(1), Assert.Single(shown.Handles[2]).Value);
        Assert.Equal(new ViewHandle(5), Assert.Single(shown.Handles[2]).Key);
    }

    [Fact]
    public void A_resumed_games_public_part_tells_everything_that_happened_on_the_way()
    {
        var host = new ScriptedHost { Numbered = true, Public = true };
        NativeRulesHost rules = NativeRulesHost.NewGame(
            host, Seats, 123, new RulesGameOptions { MinimumDeckSize = 60 }, history: [new RulesAnswer(1, new PassAnswer())], publicView: true);

        RulesStep step = rules.Start();

        PublicProjection shown = Assert.IsType<PublicProjection>(step.Public);
        Assert.Equal(2, shown.Events.Count);
        Assert.Equal(step.Events[2].Select(e => e.Sequence), shown.Events.Select(e => e.Sequence));
    }
}
