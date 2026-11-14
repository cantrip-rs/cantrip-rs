using System.Reflection;
using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using MessagePack;
using Xunit;

namespace Cantrip.Client.Tests;

public sealed class PresentationBoundaryTests
{
    [Fact]
    public void Presentation_cannot_reach_host_rules_or_content()
    {
        // What the web client draws from sees client-side state and the wire, never the server, the rules or a
        // dataset: a client that could reach them could decide what only the server may.
        Assert.Equal("Cantrip.Client.Presentation", typeof(CardActions).Assembly.GetName().Name);
        Assert.Empty(Closure(typeof(CardActions).Assembly).Except(
            ["Cantrip.Client.Presentation", "Cantrip.Client", "Cantrip.Core", "Cantrip.Protocol"], StringComparer.Ordinal));
    }

    private static IReadOnlyCollection<string> Closure(Assembly root)
    {
        HashSet<string> seen = new(StringComparer.Ordinal);
        Queue<Assembly> waiting = new([root]);
        while (waiting.TryDequeue(out Assembly? assembly))
        {
            string name = assembly.GetName().Name!;
            if (!seen.Add(name)) continue;
            foreach (AssemblyName reference in assembly.GetReferencedAssemblies()
                .Where(reference => reference.Name!.StartsWith("Cantrip.", StringComparison.Ordinal)))
                waiting.Enqueue(Assembly.Load(reference));
        }
        return seen;
    }
}

public sealed class PromptPresentationTests
{
    public static TheoryData<string> Gallery => new(PromptFixtures.Names);

    [Fact]
    public void A_lone_ability_skips_the_menu_only_when_something_after_it_can_be_cancelled()
    {
        ActionOptionMessage Ability(string cost, int targets = 0) => new()
        {
            Kind = ActionKindView.ActivateAbility,
            CostText = cost,
            TargetsRequired = targets,
            TargetCandidates = targets > 0 ? [new TargetMessage()] : [],
        };

        Assert.True(CardActions.AsksBeforeActivating(Ability("{2}, {T}")));
        Assert.True(CardActions.AsksBeforeActivating(Ability("{T}", targets: 1)));
        Assert.False(CardActions.AsksBeforeActivating(Ability("Pay 7 life")));
        Assert.False(CardActions.AsksBeforeActivating(Ability("{T}, Sacrifice this artifact")));
        Assert.False(CardActions.AsksBeforeActivating(Ability("{Q}, {E}{E}")));
    }

    /// <summary>
    /// A cost that depends on the target (CR 601.2f, 602.2b) is read out of the action's description for the targeting caption;
    /// a description with a total that does not, or none, gives nothing.
    /// </summary>
    [Theory]
    [InlineData("Equip {4} (costs {1} targeting Dwarven Mauler, {4} targeting Ornithopter)", "Costs {1} targeting Dwarven Mauler, {4} targeting Ornithopter")]
    [InlineData("costs {R} targeting Grizzly Bears, {1}{R} targeting a player", "Costs {R} targeting Grizzly Bears, {1}{R} targeting a player")]
    [InlineData("Pay 1 life, exile a blue card from your hand (costs {3})", "")]
    [InlineData("", "")]
    public void A_cost_by_target_is_read_out_of_the_actions_description(string description, string costs) =>
        Assert.Equal(costs, CardActions.CostsByTarget(new ActionOptionMessage { Description = description }));

    /// <summary>
    /// A priority of the player this seat controls (CR 723.5): the client says whose decision it is, offers no draw with it
    /// (CR 723.5b), and the automatic seat plays it as that player's own main phase - their land, from their hand.
    /// </summary>
    [Fact]
    public void A_controlled_players_priority_is_answered_as_theirs()
    {
        var transport = new FixtureTransport(PromptFixtures.Build("priority-controlling"));
        var client = new GameClient(transport);
        client.Pump();

        Assert.Equal((PromptFixtures.Rival, PromptFixtures.Rival), (client.ActingSeat, client.ControlledSeat));
        Assert.True(client.HasPriority);
        Assert.False(client.CanOfferDraw);
        Assert.Equal(4, client.Zone(ZoneKindView.Hand, PromptFixtures.Viewer).Count);
        Assert.Equal(2, client.Zone(ZoneKindView.Hand, PromptFixtures.Rival).Count);

        Assert.True(AutoPlay.Act(client));
        TakeActionAnswer played = Assert.IsType<TakeActionAnswer>(Assert.IsType<AnswerMessage>(transport.Sent.Last()).Answer);
        Assert.Equal(ActionKindView.PlayLand, played.Kind);
        Assert.Equal(PromptFixtures.Rival, client.Object(played.Source)?.Owner);
    }

    [Fact]
    public void Player_aura_board_fixture_names_its_attached_player_through_the_wire()
    {
        var client = new GameClient(new FixtureTransport(PromptFixtures.Build("player-aura")));
        client.Pump();

        ObjectView curse = Assert.Single(client.Battlefield(), card => card.Name == "Curse of the Bloody Tome");
        Assert.Equal(PromptFixtures.Viewer, curse.AttachedToPlayer);
        Assert.Equal("You", client.View!.Players.Single(player => player.Seat == curse.AttachedToPlayer).Name);
    }

    [Fact]
    public void Capped_division_fixture_carries_its_two_target_limit_through_the_client_wire()
    {
        var client = new GameClient(new FixtureTransport(PromptFixtures.Build("divide-targets-capped")));
        client.Pump();
        var prompt = Assert.IsType<ChooseDividedTargetsPromptMessage>(client.Prompt!.Prompt);
        Assert.Equal((12, 2, 3), (prompt.Total, prompt.MaxTargets, prompt.Candidates.Count));

        var remote = new GameClient(new FixtureTransport(
            [.. PromptFixtures.Build("divide-targets-capped").Select(message =>
                MessagePackSerializer.Deserialize<ServerMessage>(MessagePackSerializer.Serialize<ServerMessage>(message)))]));
        remote.Pump();
        var roundTrip = Assert.IsType<ChooseDividedTargetsPromptMessage>(remote.Prompt!.Prompt);
        Assert.Equal((12, 2, 3), (roundTrip.Total, roundTrip.MaxTargets, roundTrip.Candidates.Count));
    }

    [Theory]
    [MemberData(nameof(Gallery))]
    public void Every_fixture_survives_the_wire_and_uses_the_same_client(string name)
    {
        IReadOnlyList<ServerMessage> direct = PromptFixtures.Build(name);
        var local = new GameClient(new FixtureTransport(direct));
        var remote = new GameClient(new FixtureTransport(
            [.. direct.Select(message => MessagePackSerializer.Deserialize<ServerMessage>(
                MessagePackSerializer.Serialize<ServerMessage>(message)))]));
        local.Pump();
        remote.Pump();
        Assert.Equal(local.Seat, remote.Seat);
        Assert.Equal(local.Prompt?.Prompt.GetType(), remote.Prompt?.Prompt.GetType());
        if (local.Prompt?.Prompt is ChooseCardsPromptMessage choice)
        {
            var remoteChoice = Assert.IsType<ChooseCardsPromptMessage>(remote.Prompt!.Prompt);
            CardChoicePresentation expected = CardChoicePresentation.Describe(local, choice, []);
            Assert.Equal(expected, CardChoicePresentation.Describe(remote, remoteChoice, []));
            Assert.NotNull(remote.Prompt.Presentation);
        }
        if (local.Prompt?.Prompt is DistributeCardsPromptMessage distribution)
        {
            var roundTrip = Assert.IsType<DistributeCardsPromptMessage>(remote.Prompt!.Prompt);
            Assert.Null(roundTrip.Problem((CardsDistributedAnswer)distribution.Default()));
            Assert.Equal(distribution.Cards, roundTrip.Cards);
            Assert.Equal(
                distribution.Destinations.Select(d => (d.Kind, d.Owner, d.Min, d.Max, d.Ordered, d.Randomized)),
                roundTrip.Destinations.Select(d => (d.Kind, d.Owner, d.Min, d.Max, d.Ordered, d.Randomized)));
            Assert.Equal(local.Prompt.Presentation?.Origins.Count, remote.Prompt.Presentation?.Origins.Count);
        }
    }

    [Theory]
    [InlineData("choose-nothing", 0, true)]
    [InlineData("choose-empty", 0, true)]
    [InlineData("choose-legend", 1, false)]
    [InlineData("choose-discard", 2, false)]
    [InlineData("choose-put-onto-battlefield", 1, true)]
    [InlineData("choose-mixed-zones", 1, true)]
    public void Zero_one_many_and_optional_choices_have_correct_empty_selection(string name, int maximum, bool canSubmit)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        Assert.Equal(maximum, prompt.Count);
        var model = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Equal(canSubmit, model.CanSubmit);
        if (maximum == 0) Assert.Equal("Continue", model.Confirm);
        List<int> filled = [.. prompt.Verdict.Addable.Take(maximum).Select(handle => handle.Value)];
        Assert.True(Judged(client, prompt, filled).CanSubmit);
    }

    [Fact]
    public void Mixed_zones_do_not_get_described_as_a_hand_or_chosen_on_the_board()
    {
        GameClient client = Client("choose-mixed-zones");
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Contains("multiple zones", model.TrayTitle);
        Assert.Null(model.BoardZone);
        Assert.Equal(2, client.Prompt.Presentation!.Origins.Count);
    }

    [Fact]
    public void A_bottoming_an_effect_asks_names_its_spell_and_may_put_nothing_where_a_mulligan_names_the_rule()
    {
        GameClient effect = Client("choose-put-on-bottom-effect");
        CardChoicePresentation asked = CardChoicePresentation.Describe(
            effect, Assert.IsType<ChooseCardsPromptMessage>(effect.Prompt!.Prompt), []);
        Assert.DoesNotContain("Mulligan", asked.Title);
        Assert.Contains("choose up to 4 cards to put on the bottom", asked.Title);
        Assert.Equal("Put nothing on the bottom", asked.Confirm);
        Assert.True(asked.CanSubmit);

        GameClient mulligan = Client("choose-put-on-bottom");
        CardChoicePresentation rule = CardChoicePresentation.Describe(
            mulligan, Assert.IsType<ChooseCardsPromptMessage>(mulligan.Prompt!.Prompt), []);
        Assert.StartsWith("Mulligan:", rule.Title);
    }

    [Fact]
    public void Splice_choice_names_the_cast_action_and_the_order_of_added_text()
    {
        GameClient client = Client("choose-splice");
        ChooseCardsPromptMessage prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        CardChoicePresentation model = CardChoicePresentation.Describe(client, prompt, []);

        Assert.True(client.Prompt.MayCancel);
        Assert.Contains("splice onto this spell", model.Title);
        Assert.Contains("text follows the spell in the order chosen", model.Detail);
        Assert.Equal("Splice nothing", model.Confirm);
    }

    [Fact]
    public void Collect_evidence_counts_the_total_mana_value_and_takes_any_number_that_reaches_it()
    {
        // Collect evidence 6 (CR 701.59a) over mana values 3, 0, 4 and 1: how many is free, what they add up to is not.
        GameClient client = Client("choose-collect-evidence");
        ChooseCardsPromptMessage prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        int[] cards = [.. prompt.Verdict.Addable.Select(handle => handle.Value)];

        CardChoicePresentation none = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Contains("choose cards with total mana value 6 or more to exile to pay for it", none.Title);
        Assert.DoesNotContain("power", none.Title);
        Assert.False(none.CanSubmit);

        CardChoicePresentation short_ = Judged(client, prompt, [cards[0], cards[1], cards[3]]);
        Assert.Contains("total mana value 4 of 6", short_.Detail);
        Assert.False(short_.CanSubmit);

        CardChoicePresentation enough = Judged(client, prompt, [cards[0], cards[2]]);
        Assert.Contains("total mana value 7 of 6", enough.Detail);
        Assert.True(enough.CanSubmit);
    }

    [Fact]
    public void A_choice_says_its_rules_words_and_takes_its_verdicts_from_the_host()
    {
        // Michelangelo's Technique: up to two creature cards with total mana value 6 or less. The caption says what the rule
        // asks; whether a selection is an answer, and what may be added to it, is the host's word, asked as the cards are
        // picked (here the gallery's stand-in for it).
        GameClient client = Client("choose-put-onto-battlefield-within-total");
        var transport = (FixtureTransport)typeof(GameClient).GetField("_transport", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(client)!;
        ChooseCardsPromptMessage prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        int[] cards = [.. prompt.Verdict.Addable.Select(handle => handle.Value)];

        CardChoicePresentation none = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Contains("choose up to 2 creature cards with total mana value 6 or less to put onto the battlefield", none.Title);
        Assert.Contains("0 of 2 chosen  ·  total mana value 0 of 6 or less", none.Detail);
        Assert.True(none.CanSubmit, "the prompt's own verdict: none is an answer");
        Assert.Equal("Put nothing", none.Confirm);
        Assert.True(CardChoicePresentation.Fits(client, prompt, [], cards[1]));

        // One picked: nothing is offered or sent until the host has judged it, and it is asked once.
        CardChoicePresentation asked = CardChoicePresentation.Describe(client, prompt, [cards[0]]);
        Assert.False(asked.CanSubmit);
        Assert.False(CardChoicePresentation.Fits(client, prompt, [cards[0]], cards[1]));
        Assert.True(CardChoicePresentation.Fits(client, prompt, [cards[0]], cards[0]), "a picked card can always be put back");
        CardChoicePresentation.Describe(client, prompt, [cards[0]]);
        Assert.Single(transport.Sent.OfType<CheckSelectionMessage>(), check => check.Selected.SequenceEqual([new ViewHandle(cards[0])]));

        client.Pump();
        CardChoicePresentation four = CardChoicePresentation.Describe(client, prompt, [cards[0]]);
        Assert.True(four.CanSubmit);
        Assert.Contains("1 of 2 chosen  ·  total mana value 4 of 6 or less", four.Detail);
        Assert.True(CardChoicePresentation.Fits(client, prompt, [cards[0]], cards[1]));
    }

    [Theory]
    [InlineData("choose-return-to-hand", true)]
    [InlineData("choose-return-effect", false)]
    [InlineData("choose-sacrifice-cost", true)]
    [InlineData("choose-sacrifice-several", false)]
    public void Cost_wording_comes_from_structured_context_not_zone_or_purpose(string name, bool cost)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Equal(cost, model.Title.Contains("to pay for it", StringComparison.Ordinal));
    }

    /// <summary>
    /// Without the server's context, a return to hand or a sacrifice is a cost only where the player may back out of it, as
    /// the server judges (KI-75): Words of Wind's return pays for nothing.
    /// </summary>
    [Theory]
    [InlineData("choose-return-effect", false, false)]
    [InlineData("choose-return-effect", true, true)]
    [InlineData("choose-sacrifice-cost", false, false)]
    [InlineData("choose-sacrifice-cost", true, true)]
    public void Without_context_a_return_or_sacrifice_is_a_cost_only_when_it_can_be_backed_out_of(string name, bool mayCancel, bool cost)
    {
        GameClient client = Client(name);
        client.Prompt!.Presentation = null;
        client.Prompt.MayCancel = mayCancel;
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Equal(cost, model.Title.Contains("to pay for it", StringComparison.Ordinal));
    }

    [Fact]
    public void Replacement_order_is_not_presented_as_discarding()
    {
        GameClient client = Client("choose-order-replacements");
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Contains("replacement effects", model.Title);
        Assert.Equal("Apply next", model.Confirm);
        Assert.DoesNotContain("discard", model.Title);
    }

    /// <summary>
    /// Revealing Eye's "you may choose a nonland card from it": declining is an answer, so the button has to offer it
    /// in words. A mandatory discard counts down to its number instead, and neither reading belongs to the other.
    /// </summary>
    [Theory]
    [InlineData("choose-from-their-hand", "Discard 0", false)]
    [InlineData("choose-from-their-hand-up-to", "Discard nothing", true)]
    public void Declining_a_discard_is_only_offered_where_it_is_an_answer(string name, string confirm, bool canSubmit)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);

        Assert.Equal(confirm, model.Confirm);
        Assert.Equal(canSubmit, model.CanSubmit);
        Assert.Contains("to discard", model.Title);
        Assert.Equal("Discard it", CardChoicePresentation.Describe(
            client, prompt, [prompt.Verdict.Addable[0].Value]).Confirm);
    }

    /// <summary>
    /// Gut, True Soul Zealot's "you may sacrifice another creature or an artifact": an effect's own sacrifice, where
    /// declining is an answer. A cost's sacrifice is mandatory and counts down to its number instead.
    /// </summary>
    [Theory]
    [InlineData("choose-sacrifice", "Sacrifice 0", false)]
    [InlineData("choose-sacrifice-up-to", "Sacrifice nothing", true)]
    public void Declining_a_sacrifice_is_only_offered_where_it_is_an_answer(string name, string confirm, bool canSubmit)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);

        Assert.Equal(confirm, model.Confirm);
        Assert.Equal(canSubmit, model.CanSubmit);
        Assert.Contains("to sacrifice", model.Title);
        Assert.Equal("Sacrifice it", CardChoicePresentation.Describe(
            client, prompt, [prompt.Verdict.Addable[0].Value]).Confirm);
    }

    /// <summary>
    /// Battlefield Improvisation's "you may attach any number of Equipment you control to it": the dock says attach
    /// rather than any other purpose's verb, offers declining in words, and counts what is chosen.
    /// </summary>
    [Fact]
    public void An_attach_choice_reads_as_attaching_nothing_one_or_several()
    {
        GameClient client = Client("choose-attach");
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);

        var none = CardChoicePresentation.Describe(client, prompt, []);
        Assert.Equal("Attach nothing", none.Confirm);
        Assert.True(none.CanSubmit);
        Assert.Contains("choose any number of permanents to attach", none.Title);
        Assert.Equal(ZoneKindView.Battlefield, none.BoardZone);
        Assert.False(none.AnswersOnLastPick);

        Assert.Equal("Attach it", CardChoicePresentation.Describe(client, prompt, [prompt.Verdict.Addable[0].Value]).Confirm);
        Assert.Equal("Attach 2", CardChoicePresentation.Describe(
            client, prompt, [prompt.Verdict.Addable[0].Value, prompt.Verdict.Addable[1].Value]).Confirm);

        GameClient single = Client("choose-attach-one");
        var one = Assert.IsType<ChooseCardsPromptMessage>(single.Prompt!.Prompt);
        Assert.Contains("choose any number of permanents to attach", CardChoicePresentation.Describe(single, one, []).Title);
    }

    /// <summary>
    /// One rule for choosing permanents on the battlefield, the same as for targets: exactly so many answers on the click
    /// that reaches the count, with no button to confirm; "up to" so many keeps its button, since any count is an answer.
    /// </summary>
    [Theory]
    [InlineData("choose-sacrifice", true)]
    [InlineData("choose-sacrifice-up-to", false)]
    public void An_exact_board_choice_answers_on_its_last_pick_and_an_up_to_one_waits(string name, bool answersOnLastPick)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        var model = CardChoicePresentation.Describe(client, prompt, []);

        Assert.Equal(ZoneKindView.Battlefield, model.BoardZone);
        Assert.Equal(answersOnLastPick, model.AnswersOnLastPick);
    }

    /// <summary>
    /// Urza, Lord High Artificer is offered once per artifact it could tap. Those are one menu entry, not a list of
    /// names: which artifact is picked on the board afterwards, like a target, and auto-mana never picks it.
    /// </summary>
    [Fact]
    public void A_chosen_tap_cost_is_one_entry_that_reads_without_the_permanent()
    {
        GameClient client = Client("priority-tap-for-mana");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        List<ActionOptionMessage> urza =
            [.. priority.Options.Where(option => client.Object(option.Source)?.Name == "Urza, Lord High Artificer")];
        Assert.Equal(2, urza.Count);

        var entry = Assert.Single(CardActions.MenuEntries(CardActions.ManaRows(urza)));
        Assert.Equal(2, entry.Count);
        Assert.Equal(2, entry.Select(row => CardActions.CostPermanent(row)).Distinct().Count());
        Assert.Equal("tap", CardActions.CostVerb(entry[0].Option));
        Assert.True(CardActions.NeedsAChoice(entry[0].Option));

        string reading = CardActions.Describe(client, entry[0].Option, anyPermanent: true);
        Assert.Equal("Tap an untapped artifact you control: Add {U}...", reading);
    }

    /// <summary>
    /// Squandered Resources: one option per land it could sacrifice, each adding what that land could. One entry, the
    /// land picked on the board; a lone candidate is still picked rather than taken silently.
    /// </summary>
    [Fact]
    public void A_chosen_sacrifice_cost_is_one_entry_whatever_each_permanent_would_add()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        List<ActionOptionMessage> resources =
            [.. priority.Options.Where(option => client.Object(option.Source)?.Name == "Squandered Resources")];

        var entry = Assert.Single(CardActions.MenuEntries(CardActions.ManaRows(resources)));
        Assert.Equal(2, entry.Select(row => CardActions.CostPermanent(row)).Distinct().Count());
        Assert.Equal("sacrifice", CardActions.CostVerb(entry[0].Option));
        Assert.All(resources, option => Assert.True(CardActions.NeedsAChoice(option)));
    }

    /// <summary>
    /// Karn, Scion of Urza's +1: "An opponent chooses one of them. Put that card into your hand." The chooser is not
    /// the one who gets the card, so the dock that says "your hand" to Barrowgoyf's own controller must not say it
    /// here.
    /// </summary>
    [Theory]
    [InlineData("choose-put-into-hand", "your hand", "Put into your hand")]
    [InlineData("choose-put-into-their-hand", "its owner's hand", "Put into their hand")]
    public void A_card_chosen_for_somebody_elses_hand_does_not_offer_it_to_the_chooser(
        string name, string whose, string confirm)
    {
        GameClient client = Client(name);
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);

        Assert.Contains($"to put into {whose}", CardChoicePresentation.Describe(client, prompt, []).Title);
        Assert.Equal(confirm, CardChoicePresentation.Describe(
            client, prompt, [prompt.Verdict.Addable[0].Value]).Confirm);
    }

    [Fact]
    public void Divided_target_order_distinguishes_players_and_objects_and_reselection()
    {
        var state = new PromptSelection();
        var player = new TargetMessage { Player = 1 };
        var permanent = new TargetMessage { Object = new ViewHandle(1) };
        state.SetDividedShare(player, 1);
        state.SetDividedShare(permanent, 2);
        state.SetDividedShare(player, 0);
        state.SetDividedShare(player, 1);
        Assert.Equal([permanent, player], state.DividedOrder);
        Assert.Equal(2, state.DividedShares[1]);
        Assert.Equal(1, state.DividedPlayerShares[1]);
        state.Clear();
        Assert.Empty(state.DividedOrder);
        Assert.Empty(state.DividedShares);
        Assert.Empty(state.DividedPlayerShares);
    }

    [Fact]
    public void Selection_limits_replacement_toggling_and_new_prompt_reset_are_local_state()
    {
        var state = new PromptSelection();
        state.Pick(10, 0);
        Assert.Empty(state.Cards);
        state.Pick(10, 1);
        state.Pick(20, 1);
        Assert.Equal([20], state.Cards);
        state.Pick(20, 1);
        Assert.Empty(state.Cards);
        state.Pick(10, 2);
        state.Pick(20, 2);
        state.Pick(30, 2);
        Assert.Equal([10, 20], state.Cards);
        state.Placements = [[10], [20]];
        state.Clear();
        Assert.Empty(state.Cards);
        Assert.Null(state.Placements);
    }

    /// <summary>
    /// Stack All answers with the order the box is showing, and nothing else: the triggers already placed, then the
    /// rest as they are laid out. First in the answer goes on the stack first and resolves last, so the order the
    /// player reads left to right is the order the stack ends up in, bottom to top.
    /// </summary>
    [Fact]
    public void Stacking_the_triggers_as_shown_is_the_order_clicking_them_would_have_sent()
    {
        var state = new PromptSelection();

        // Nothing placed: the order shown is the order they triggered in.
        Assert.Equal([0, 1, 2], state.TriggerOrderAsShown(3));
        Assert.Equal([0, 1, 2], state.TriggersLeft(3));

        // One placed by hand: it keeps its place, and the rest follow in the order the box still shows them.
        state.TriggerOrder.Add(2);
        Assert.Equal([0, 1], state.TriggersLeft(3));
        Assert.Equal([2, 0, 1], state.TriggerOrderAsShown(3));

        // Clicking the rest left to right sends exactly what the button would have.
        foreach (int index in state.TriggersLeft(3).ToList())
        {
            state.TriggerOrder.Add(index);
        }

        Assert.Equal([2, 0, 1], state.TriggerOrder);
        Assert.Empty(state.TriggersLeft(3));
        Assert.Equal([2, 0, 1], state.TriggerOrderAsShown(3));

        state.Clear();
        Assert.Equal([0, 1], state.TriggerOrderAsShown(2));
    }

    [Fact]
    public void Unknown_duplicate_or_excess_cards_cannot_enable_confirmation()
    {
        GameClient client = Client("choose-discard");
        var prompt = Assert.IsType<ChooseCardsPromptMessage>(client.Prompt!.Prompt);
        int first = prompt.Verdict.Addable[0].Value;
        Assert.False(Judged(client, prompt, [first, first]).CanSubmit);
        Assert.False(Judged(client, prompt, [first, int.MaxValue]).CanSubmit);
    }

    /// <summary>
    /// CR 715.3: casting an adventurer card as its Adventure casts Battle Display, so the menu row says that and
    /// nothing else — not "Cast Embereth Shieldbreaker as its Adventure, Battle Display". The other reasons a cast
    /// carries a description, an alternative cost or a card cast from somewhere other than a hand, still say so.
    /// </summary>
    [Fact]
    public void An_adventure_reads_as_the_adventures_own_name()
    {
        GameClient client = Client("priority");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        ActionOptionMessage plain = priority.Options.First(option =>
            option.Kind == ActionKindView.CastSpell && option.TargetsRequired == 0 && !option.HasX);
        string card = client.Object(plain.Source)!.Name;

        Assert.Equal($"Cast {card}", CardActions.Describe(client, plain));

        Assert.Equal("Cast Battle Display...", CardActions.Describe(client, new ActionOptionMessage
        {
            Kind = ActionKindView.CastSpell,
            Source = plain.Source,
            AbilityIndex = -1,
            Affordable = true,
            TargetsRequired = 1,
            TargetCandidates = [new TargetMessage { Object = plain.Source }],
            Description = "as its Adventure, Battle Display {R}",
            SpellName = "Battle Display",
        }));

        // CR 720.3: an omen card cast as its Omen casts Charring Bite, whose own cost the server sends, since the creature
        // card in the hand does not show it.
        Assert.Equal("Cast Charring Bite {1}{R}...", CardActions.Describe(client, new ActionOptionMessage
        {
            Kind = ActionKindView.CastSpell,
            Source = plain.Source,
            AbilityIndex = 249,
            Affordable = true,
            TargetsRequired = 1,
            TargetCandidates = [new TargetMessage { Object = plain.Source }],
            Description = "Charring Bite",
            SpellName = "Charring Bite",
            CostText = "{1}{R}",
        }));

        Assert.Equal($"Cast {card} — from exile for {{2}}", CardActions.Describe(client, new ActionOptionMessage
        {
            Kind = ActionKindView.CastSpell,
            Source = plain.Source,
            AbilityIndex = 1,
            Affordable = true,
            Description = "from exile for {2}",
        }));
    }

    /// <summary>
    /// A mana ability that adds mana "in any combination of" some colours (CR 106.1b) reads as one menu row per
    /// combination, and a cost that sacrifices one of several permanents names the one each row would take. Pinned
    /// here because neither wording is reachable from a fixture, and a row that read "Add 3 of {R}/{G}" would be a
    /// lie about a card that can make {R}{R}{G}.
    /// </summary>
    /// <summary>
    /// Squandered Resources offers one option per sacrifice candidate (CR 106.7), and a Strip Mine's is a choice of
    /// colorless alone: a choice with no colours listed. It must still be a row — one reading {C}, answered with the
    /// mana type — or a player could never take the {C} the card allows.
    /// </summary>
    [Fact]
    public void Finite_mana_bundles_are_one_action_with_all_outcomes_visible()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            CostText = "{T}",
            ProducesAlternatives = [[ManaTypeView.Blue, ManaTypeView.Blue], [ManaTypeView.Blue, ManaTypeView.Blue, ManaTypeView.Colorless, ManaTypeView.Colorless]],
            Description = "{T}: Add {U} or {C}{U}.",
        };
        Assert.Single(CardActions.ManaRows([option]));
        Assert.Contains("Add {U}{U} or {U}{U}{C}{C}", CardActions.Describe(client, option));
        Assert.True(AutoMana.IsManaAbility(option));
        var decoded = MessagePackSerializer.Deserialize<ActionOptionMessage>(MessagePackSerializer.Serialize(option));
        Assert.Equal(option.ProducesAlternatives[1], decoded.ProducesAlternatives[1]);
        option.ProducesAlternatives = [[ManaTypeView.Blue], [ManaTypeView.Colorless]];
        Assert.True(AutoMana.IsManaAbility(option));
        Assert.False(CardActions.IsIrreversibleActivation(option));
        Assert.Contains("Add {U} or {C}", CardActions.Describe(client, option));
    }

    [Fact]
    public void A_color_choice_remains_selectable_when_every_choice_adds_the_same_mana()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Produces = [ManaTypeView.Colorless],
            ColorChoices = [ColorView.White, ColorView.Blue, ColorView.Red],
            Description = "{T}: Add three mana of any one color. (can add {C} now)",
        };
        var rows = CardActions.ManaRows([option]);
        Assert.Equal(3, rows.Count);
        var red = Assert.Single(rows, row => row.Color == ColorView.Red);
        Assert.Contains("Choose {R} → {C}", CardActions.Describe(client, option, red.Color));
        Assert.Equal(ColorView.Red, AutoPlay.Answer(option, ColorView.Red).Color);
        Assert.Equal(ColorView.White, AutoPlay.Answer(option).Color);

        option.Produces.Clear();
        Assert.Equal(3, CardActions.ManaRows([option]).Count);
        Assert.Contains("Choose {R} → no mana", CardActions.Describe(client, option, ColorView.Red));
        Assert.Equal(ColorView.Red, AutoPlay.Answer(option, ColorView.Red).Color);
        Assert.False(CardActions.IsIrreversibleActivation(option));
        Assert.False(AutoPass.CanAct(new PriorityPromptMessage { Options = [option] }));
        option.Description = string.Empty;
        option.CostText = "{T}";
        Assert.Equal("{T}: Choose {R} → no mana", CardActions.Describe(client, option, ColorView.Red));
    }

    [Fact]
    public void Mana_action_reads_as_its_cost_and_mana_with_its_spending_restriction()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Produces = [ManaTypeView.Colorless],
            CostText = "{T}",
            Description = "{T}: Add {C}. Spend this mana only on artifact spells.",
            Filter = ManaSpendFilterView.NotNonartifactSpells,
        };
        string described = CardActions.Describe(client, option);
        Assert.StartsWith("{T}: Add {C}", described);
        Assert.Contains("can't be spent on nonartifact spells", described);
    }

    [Fact]
    public void Cumulative_upkeep_mana_action_names_its_restriction()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var option = new ActionOptionMessage { Kind = ActionKindView.ActivateAbility, Produces = [ManaTypeView.Blue], CostText = "{T}", Filter = ManaSpendFilterView.CumulativeUpkeep };
        Assert.Contains("only for cumulative upkeep costs", CardActions.Describe(client, option));
    }

    [Fact]
    public void A_mana_ability_the_player_has_of_their_own_is_theirs_and_not_a_cards()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var channel = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = ViewHandle.None,
            AbilityIndex = 1 << 20,
            Produces = [ManaTypeView.Colorless],
            Description = "Pay 1 life: Add {C}.",
            CostText = "Pay 1 life",
            LifeCost = 1,
            GrantedBy = "Channel",
        };

        // CR 611.2a: no card offers it, so the player's own area does; it still reads as a mana ability does.
        var actions = new CardActions(new PriorityPromptMessage { Player = 1, Options = [channel] });
        Assert.Same(channel, Assert.Single(actions.Own));
        Assert.Equal("Pay 1 life: Add {C}", CardActions.Describe(client, channel));
        Assert.Empty(new CardActions(Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt)).Own);
    }

    [Fact]
    public void A_colorless_only_choice_is_a_row_that_adds_colorless()
    {
        GameClient client = Client("priority-sacrifice-for-mana");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        List<ActionOptionMessage> resources =
            [.. priority.Options.Where(option => client.Object(option.Source)?.Name == "Squandered Resources")];

        var rows = CardActions.ManaRows(resources);

        Assert.Equal(2, rows.Count);
        var colorless = Assert.Single(rows, row => row.Color is null);
        Assert.True(CardActions.ChoosesColorless(colorless.Option));
        Assert.EndsWith(
            ": Add {C}",
            CardActions.Describe(client, colorless.Option, colorless.Color, colorless.Combination, "Strip Mine"));

        var black = Assert.Single(rows, row => row.Color == ColorView.Black);
        Assert.False(CardActions.ChoosesColorless(black.Option));
    }

    [Fact]
    public void An_any_combination_ability_reads_as_the_mana_each_row_would_add()
    {
        GameClient client = Client("priority");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        ViewHandle source = priority.Options[0].Source;

        var lumberjack = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = source,
            Affordable = true,
            CostText = "{T}, Sacrifice a Forest",
            ProducesOfAnyColorEach = 3,
            ColorChoices = [ColorView.Red, ColorView.Green],
            SacrificeCandidates = [source],
        };

        // Unchosen — how the ability reads before a row narrows it.
        Assert.Equal(
            "{T}, Sacrifice a Forest: Add 3 in any combination of {R}/{G}...",
            CardActions.Describe(client, lumberjack));

        // One row per combination, reading as the mana it would actually add.
        Assert.Equal(
            "{T}, Sacrifice a Forest: Add {R}{R}{G}",
            CardActions.Describe(client, lumberjack, combination: [ColorView.Red, ColorView.Red, ColorView.Green]));

        // With more than one permanent able to pay, the row says which it takes.
        Assert.Equal(
            $"{{T}}, Sacrifice a Forest ({client.Object(source)!.Name}): Add {{G}}{{G}}{{G}}",
            CardActions.Describe(
                client,
                lumberjack,
                combination: [ColorView.Green, ColorView.Green, ColorView.Green],
                sacrificed: client.Object(source)!.Name));
    }

    /// <summary>
    /// The combinations a menu offers are multisets, not orderings: {R}{G} and {G}{R} are the same mana, so offering
    /// both would be two rows that do the same thing.
    /// </summary>
    [Fact]
    public void The_offered_combinations_are_one_per_multiset()
    {
        List<List<ColorView>> two = CardActions.ColorCombinations([ColorView.Red, ColorView.Green], 3);
        Assert.Equal(4, two.Count);

        // The uniform ones first, which is what a player reaching for "all of one colour" wants at the top.
        Assert.Equal([ColorView.Red, ColorView.Red, ColorView.Red], two[0]);
        Assert.Equal([ColorView.Green, ColorView.Green, ColorView.Green], two[1]);

        Assert.Equal(4, two.Select(c => string.Join("", c)).Distinct().Count());

        // Relic of Sauron's two of three colours: three uniform and three mixed.
        Assert.Equal(6, CardActions.ColorCombinations([ColorView.Blue, ColorView.Black, ColorView.Red], 2).Count);
    }

    /// <summary>A choice described once the host has judged the cards picked: described, its check answered, described again.</summary>
    private static CardChoicePresentation Judged(GameClient client, ChooseCardsPromptMessage prompt, IReadOnlyList<int> picked)
    {
        CardChoicePresentation.Describe(client, prompt, picked);
        client.Pump();
        return CardChoicePresentation.Describe(client, prompt, picked);
    }

    private static GameClient Client(string fixture)
    {
        var client = new GameClient(new FixtureTransport(PromptFixtures.Build(fixture)));
        client.Pump();
        return client;
    }

    [Fact]
    public void Chosen_tap_rows_name_the_permanent_and_the_bot_returns_its_handle()
    {
        GameClient client = Client("priority");
        var priority = Assert.IsType<PriorityPromptMessage>(client.Prompt!.Prompt);
        var source = priority.Options[0].Source;
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility, Source = source, Tap = source,
            CostText = "Tap an untapped artifact you control", Produces = [ManaTypeView.Blue],
        };
        Assert.Contains($"({client.Object(source)!.Name}): Add {{U}}", CardActions.Describe(client, option));
        Assert.Equal(source, AutoPlay.Answer(option).Tap);
    }
}
