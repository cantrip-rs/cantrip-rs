using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Llm.Tests.Board;

namespace Cantrip.Client.Llm.Tests;

/// <summary>
/// What a prompt turns into, and what comes back out.
/// <para>
/// Two things are being proved throughout, and only the first is about the questions. One: a prompt becomes
/// questions a reader could answer, and a question nobody needs to answer is not asked — every one of those is a
/// call not paid for. Two, and the one that matters: whatever is chosen, the answer is built only out of what the
/// prompt offered, so a model that answers badly plays badly and cannot play illegally.
/// </para>
/// </summary>
public sealed class DecisionTests
{
    // --- Priority ---------------------------------------------------------------------------------------------------

    [Fact]
    public void Passes_without_asking_when_there_is_nothing_worth_doing()
    {
        // The saving that makes the whole thing affordable. Most priority in a game is an empty hand or an uncastable
        // one, and none of it is worth a round trip to say "pass" about.
        var chooser = new ScriptedChooser();
        var prompt = new PriorityPromptMessage
        {
            Player = Me,
            Options =
            [
                // A mana ability, which is never taken at priority, and a spell that cannot be paid for.
                new() { Kind = ActionKindView.ActivateAbility, Source = Handle(1), Affordable = true, Produces = [ManaTypeView.Red] },
                new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = false },
            ],
        };

        AnswerPayload answer = new Decisions(chooser).Answer(Seated(), prompt);

        Assert.IsType<PassAnswer>(answer);
        Assert.Empty(chooser.Asked);
    }

    [Fact]
    public void Offers_passing_as_a_choice_of_its_own()
    {
        // Passing is a decision (CR 117.3d), not what happens when nothing is picked: holding a counterspell and
        // leaving a blocker up are both this choice.
        var chooser = new ScriptedChooser([1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(2, "Lightning Bolt", "{R}")),
            new PriorityPromptMessage
            {
                Player = Me,
                Options = [new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true }],
            });

        Assert.IsType<PassAnswer>(answer);
        Assert.Single(chooser.Asked);

        // The spell, then the ways of not casting it: pass and be asked again, or wait for a named point.
        IReadOnlyList<string> choices = chooser.Asked[0].Choices;
        Assert.Contains("Pass for now", choices[1], StringComparison.Ordinal);
        Assert.All(choices.Skip(2), choice => Assert.StartsWith("Wait ", choice, StringComparison.Ordinal));
    }

    [Fact]
    public void Takes_the_action_it_was_told_to_and_names_nothing_that_was_not_offered()
    {
        var chooser = new ScriptedChooser([0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(2, "Lightning Bolt", "{R}", "Lightning Bolt deals 3 damage to any target.")),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true, AbilityIndex = 0 },
                    new() { Kind = ActionKindView.PlayLand, Source = Handle(3) },
                ],
            });

        var action = Assert.IsType<TakeActionAnswer>(answer);
        Assert.Equal(ActionKindView.CastSpell, action.Kind);
        Assert.Equal(Handle(2), action.Source);

        // The card's own text goes in the choice: a model that has never heard of the card still knows what it does.
        Assert.Contains("3 damage", chooser.Asked[0].Choices[0], StringComparison.Ordinal);
    }

    [Fact]
    public void Asks_what_a_spell_is_aimed_at_after_it_has_been_chosen()
    {
        var chooser = new ScriptedChooser([0], [1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(
                Card(2, "Lightning Bolt", "{R}"),
                Creature(5, "Grizzly Bears", 2, 2),
                Creature(6, "Serra Angel", 4, 4, Them)),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new()
                    {
                        Kind = ActionKindView.CastSpell,
                        Source = Handle(2),
                        Affordable = true,
                        TargetsRequired = 1,
                        TargetCandidates = [new() { Object = Handle(5) }, new() { Object = Handle(6) }],
                    },
                ],
            });

        var action = Assert.IsType<TakeActionAnswer>(answer);
        Assert.Equal(Handle(6), Assert.Single(action.Targets).Object);
        Assert.Equal(2, chooser.Asked.Count);
        Assert.True(chooser.Spent);
    }

    [Fact]
    public void Does_not_ask_about_a_target_there_is_only_one_of()
    {
        var chooser = new ScriptedChooser([0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(2, "Lightning Bolt", "{R}"), Creature(6, "Serra Angel", 4, 4, Them)),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new()
                    {
                        Kind = ActionKindView.CastSpell,
                        Source = Handle(2),
                        Affordable = true,
                        TargetsRequired = 1,
                        TargetCandidates = [new() { Object = Handle(6) }],
                    },
                ],
            });

        Assert.Equal(Handle(6), Assert.Single(Assert.IsType<TakeActionAnswer>(answer).Targets).Object);
        Assert.Single(chooser.Asked);
    }

    [Fact]
    public void Offers_aiming_at_nothing_only_where_the_prompt_allows_it()
    {
        var chooser = new ScriptedChooser([1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(6, "Serra Angel", 4, 4, Them)),
            new ChooseTargetPromptMessage
            {
                Player = Me,
                Ability = Handle(9),
                Text = "you may exile target creature",
                Optional = true,
                Candidates = [new() { Object = Handle(6) }],
            });

        Assert.Empty(Assert.IsType<TargetsChosenAnswer>(answer).Targets);
        Assert.Contains("nothing", chooser.Asked[0].Choices[1], StringComparison.OrdinalIgnoreCase);
    }

    // --- Combat ------------------------------------------------------------------------------------------------------

    [Fact]
    public void Declares_attackers_as_one_question_when_there_is_one_player_to_attack()
    {
        // Six creatures against one opponent is one question with six choices, not sixty-four combinations and not
        // six questions.
        var chooser = new ScriptedChooser([0, 2]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(
                Creature(1, "Grizzly Bears", 2, 2),
                Creature(2, "Llanowar Elves", 1, 1),
                Creature(3, "Serra Angel", 4, 4)),
            new DeclareAttackersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1), Handle(2), Handle(3)],
                AttackablePlayers = [Them],
            });

        var attacks = Assert.IsType<AttackersDeclaredAnswer>(answer);
        Assert.Single(chooser.Asked);
        Assert.Equal(2, attacks.Attacks.Count);
        Assert.Equal([Handle(1), Handle(3)], attacks.Attacks.Select(attack => attack.Attacker));
        Assert.All(attacks.Attacks, attack => Assert.Equal(Them, attack.TargetPlayer));
    }

    [Fact]
    public void Tells_an_attacker_what_attacking_costs_and_who_could_block()
    {
        // Neither fact can be read off the board, and both change whether an attack is good: a creature that attacks
        // is tapped and so is not there to block, and blocks are chosen after the attack is declared. A seat not
        // told them is deciding combat without them, which is what sending a 1/2 into a 2/2 looks like from outside.
        var chooser = new ScriptedChooser([[]]);

        new Decisions(chooser).Answer(
            Seated(Creature(1, "Mother of Runes", 1, 2), Creature(9, "Grizzly Bears", 2, 2, Them)),
            new DeclareAttackersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1)],
                AttackablePlayers = [Them],
            });

        string note = chooser.Asked[0].Note;
        Assert.Contains("becomes tapped", note, StringComparison.Ordinal);
        Assert.Contains("cannot block", note, StringComparison.Ordinal);
        Assert.Contains("declared after attackers", note, StringComparison.Ordinal);

        // And what it would be attacking into, with the numbers the decision turns on.
        Assert.Contains("Grizzly Bears #9 2/2", note, StringComparison.Ordinal);
    }

    [Fact]
    public void Asks_for_a_reason_before_the_letter()
    {
        // Worth a few dozen output tokens against the thousands the question costs: a model that states the tradeoff
        // tends to choose better, and a game that went badly is otherwise unaccountable.
        string asked = new Question
        {
            Heading = "What do you do?",
            Choices = ["Attack.", "Pass."],
        }.ToString();

        Assert.Contains("one short line saying why", asked, StringComparison.Ordinal);
        Assert.Contains("on the last line", asked, StringComparison.Ordinal);
    }

    [Fact]
    public void Declares_a_creature_that_must_attack_without_asking_about_it()
    {
        // A goaded creature (CR 701.15b) has no choice to make, so it is not a question — but it is still declared,
        // because a declaration leaving it out is refused whole.
        var chooser = new ScriptedChooser([[]]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(1, "Grizzly Bears", 2, 2), Creature(2, "Goaded Ogre", 3, 3)),
            new DeclareAttackersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1), Handle(2)],
                AttackablePlayers = [Them],
                Requirements = [new() { Attacker = Handle(2) }],
                MostRequirements = 1,
                Suggested = [new AttackMessage { Attacker = Handle(2), TargetPlayer = Them }],
                Verdict = new AttackVerdictView
                {
                    Addable = [new AttackMessage { Attacker = Handle(1), TargetPlayer = Them }, new AttackMessage { Attacker = Handle(2), TargetPlayer = Them }],
                },
            });

        var attacks = Assert.IsType<AttackersDeclaredAnswer>(answer);
        Assert.Equal(Handle(2), Assert.Single(attacks.Attacks).Attacker);

        // Asked only about the one that had a choice, and told about the one that did not.
        Assert.Single(chooser.Asked);
        Assert.Single(chooser.Asked[0].Choices);
        Assert.Contains("Goaded Ogre", chooser.Asked[0].Note, StringComparison.Ordinal);
    }

    [Fact]
    public void Asks_per_creature_once_there_is_more_than_one_thing_to_attack()
    {
        var chooser = new ScriptedChooser([2], [1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(
                Creature(1, "Grizzly Bears", 2, 2),
                Creature(2, "Llanowar Elves", 1, 1),
                Creature(7, "Chandra", 0, 0, Them)),
            new DeclareAttackersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1), Handle(2)],
                AttackablePlayers = [Them],
                AttackablePlaneswalkers = [Handle(7)],
            });

        var attacks = Assert.IsType<AttackersDeclaredAnswer>(answer);
        Assert.Equal(2, chooser.Asked.Count);

        // The first stayed home; the second went at the planeswalker.
        AttackMessage attack = Assert.Single(attacks.Attacks);
        Assert.Equal(Handle(2), attack.Attacker);
        Assert.Equal(Handle(7), attack.TargetPermanent);
    }

    [Fact]
    public void Blocks_only_with_pairs_the_prompt_allowed()
    {
        var chooser = new ScriptedChooser([0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(1, "Wall of Wood", 0, 3), Creature(9, "Serra Angel", 4, 4, Them)),
            new DeclareBlockersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1), Handle(2)],
                Attackers = [Handle(9)],
                Allowed = [new() { Blocker = Handle(1), Attacker = Handle(9) }],
                MinimumBlockers = [1],
            });

        BlockMessage block = Assert.Single(Assert.IsType<BlockersDeclaredAnswer>(answer).Blocks);
        Assert.Equal(Handle(1), block.Blocker);

        // The creature with no allowed pair was never offered, so it can never be sent.
        Assert.Single(chooser.Asked);
        Assert.Single(chooser.Asked[0].Choices);
    }

    [Fact]
    public void A_creature_that_can_block_several_attackers_is_asked_which()
    {
        // "Can block any number of creatures" (CR 509.1a): one question for the creature, any of the attackers it may block,
        // each chosen a block of its own.
        var chooser = new ScriptedChooser([0, 2]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(
                Creature(1, "Wall of Glare", 0, 5),
                Creature(7, "Grizzly Bears", 2, 2, Them),
                Creature(8, "Raging Goblin", 1, 1, Them),
                Creature(9, "Savannah Lions", 2, 1, Them)),
            new DeclareBlockersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1)],
                Attackers = [Handle(7), Handle(8), Handle(9)],
                Allowed =
                [
                    new() { Blocker = Handle(1), Attacker = Handle(7) },
                    new() { Blocker = Handle(1), Attacker = Handle(8) },
                    new() { Blocker = Handle(1), Attacker = Handle(9) },
                ],
                MinimumBlockers = [1, 1, 1],
                BlockCapacities = [0],
            });

        List<BlockMessage> blocks = Assert.IsType<BlockersDeclaredAnswer>(answer).Blocks;
        Assert.Equal([Handle(7), Handle(9)], blocks.Select(block => block.Attacker));
        Assert.All(blocks, block => Assert.Equal(Handle(1), block.Blocker));
        Question asked = Assert.Single(chooser.Asked);
        Assert.Contains("any number of creatures", asked.Heading, StringComparison.Ordinal);
        Assert.Equal(3, asked.Picks);
    }

    [Fact]
    public void Takes_back_a_block_that_is_one_short_of_the_minimum()
    {
        // Menace is stated per attacker and a declaration one blocker short of it is refused whole (CR 509.1b), which
        // would throw away every other block with it. The minimum is in the question too, so it can be met on purpose.
        var chooser = new ScriptedChooser([0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(1, "Grizzly Bears", 2, 2), Creature(2, "Wall of Wood", 0, 3), Creature(9, "Sneaky Ogre", 3, 3, Them)),
            new DeclareBlockersPromptMessage
            {
                Player = Me,
                Eligible = [Handle(1), Handle(2)],
                Attackers = [Handle(9)],
                Allowed = [new() { Blocker = Handle(1), Attacker = Handle(9) }, new() { Blocker = Handle(2), Attacker = Handle(9) }],
                MinimumBlockers = [2],
            });

        Assert.Empty(Assert.IsType<BlockersDeclaredAnswer>(answer).Blocks);
        Assert.Contains("fewer than 2", chooser.Asked[0].Note, StringComparison.Ordinal);
    }

    [Fact]
    public void Divides_combat_damage_only_in_ways_the_rules_allow()
    {
        // The legal divisions are a short list: lethal to each blocker in order before anything reaches the next one,
        // and only once every one of them has lethal may the rest trample through (CR 510.1c-d, 702.19b).
        var chooser = new ScriptedChooser([2]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(
                Creature(1, "Ball Lightning", 6, 1),
                Creature(8, "Grizzly Bears", 2, 2, Them),
                Creature(9, "Wall of Wood", 0, 3, Them)),
            new AssignCombatDamagePromptMessage
            {
                Player = Me,
                Divisions =
                [
                    new()
                    {
                        Source = Handle(1),
                        Damage = 6,
                        Creatures = [Handle(8), Handle(9)],
                        Lethal = [2, 3],
                        MayAssignToDefender = true,
                        Defender = Them,
                        Suggested = [2, 3],
                        SuggestedToDefender = 1,
                    },
                ],
            });

        var assigned = Assert.IsType<CombatDamageAssignedAnswer>(answer);
        Assert.Equal(6, assigned.Shares.Sum(share => share.Amount));

        // Three ways: all six into the first blocker; lethal to the first and the rest into the second; lethal to both
        // and the last point through. The third was chosen.
        Assert.Equal(3, chooser.Asked[0].Choices.Count);
        Assert.Equal(1, assigned.Shares.Single(share => share.Creature == ViewHandle.None).Amount);
        Assert.Equal(2, assigned.Shares.Single(share => share.Creature == Handle(8)).Amount);
        Assert.Equal(3, assigned.Shares.Single(share => share.Creature == Handle(9)).Amount);
    }

    [Fact]
    public void Never_tramples_past_a_blocker_that_is_short_of_lethal()
    {
        var chooser = new FirstChooser();

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(1, "Trampler", 3, 3), Creature(8, "Wall of Wood", 0, 5, Them)),
            new AssignCombatDamagePromptMessage
            {
                Player = Me,
                Divisions =
                [
                    new()
                    {
                        Source = Handle(1),
                        Damage = 3,
                        Creatures = [Handle(8)],
                        Lethal = [5],
                        MayAssignToDefender = true,
                        Defender = Them,
                        Suggested = [3],
                    },
                ],
            });

        // Three damage does not cover the wall's five, so nothing may go past it: the only division is all of it into
        // the blocker, and with one way there is nothing to ask about.
        var assigned = Assert.IsType<CombatDamageAssignedAnswer>(answer);
        Assert.Equal(Handle(8), Assert.Single(assigned.Shares).Creature);
        Assert.Equal(3, assigned.Shares[0].Amount);
        Assert.Equal(0, chooser.Questions);
    }

    // --- Choosing ----------------------------------------------------------------------------------------------------

    [Fact]
    public void Divides_a_total_among_the_targets_it_chose()
    {
        var chooser = new ScriptedChooser([0], [0], [2], [1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(5, "Grizzly Bears", 2, 2, Them), Creature(6, "Serra Angel", 4, 4, Them)),
            new ChooseDividedTargetsPromptMessage
            {
                Player = Me,
                Ability = Handle(9),
                Text = "deals 4 damage divided as you choose among any number of targets",
                Total = 4,
                Candidates = [new() { Object = Handle(5) }, new() { Object = Handle(6) }],
            });

        var divided = Assert.IsType<TargetsDividedAnswer>(answer);
        Assert.Equal(2, divided.Shares.Count);
        Assert.Equal(4, divided.Shares.Sum(share => share.Amount));

        // Every chosen target gets at least one of the total (CR 601.2d), which is what makes the splits a short list.
        Assert.All(divided.Shares, share => Assert.True(share.Amount >= 1));
    }

    [Fact]
    public void Aims_an_up_to_division_at_nothing_when_it_says_so()
    {
        // The first question offers the two creatures and then "Aim it at nothing.": the third choice.
        var chooser = new ScriptedChooser([2]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(5, "Grizzly Bears", 2, 2, Them), Creature(6, "Serra Angel", 4, 4, Them)),
            new ChooseDividedTargetsPromptMessage
            {
                Player = Me,
                Ability = Handle(9),
                Text = "+1: Distribute two +1/+1 counters among up to two target creatures. They gain trample until end of turn.",
                Total = 2,
                MaxTargets = 2,
                Optional = true,
                Candidates = [new() { Object = Handle(5) }, new() { Object = Handle(6) }],
            });

        Assert.Empty(Assert.IsType<TargetsDividedAnswer>(answer).Shares);
    }

    [Fact]
    public void Removes_counters_for_a_cost_one_question_per_counter_naming_kind_and_creature()
    {
        // The Ballista's one counter is taken first; then only the Caryatid's are left, which needs no question.
        var chooser = new ScriptedChooser([1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(5, "Sylvan Caryatid", 0, 1), Creature(6, "Walking Ballista", 1, 1)),
            new RemoveCountersPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Total = 3,
                Pools =
                [
                    new CounterPoolMessage { Object = Handle(5), Counter = "-1/-1", Count = 2 },
                    new CounterPoolMessage { Object = Handle(6), Counter = "+1/+1", Count = 1 },
                ],
            });

        var removed = Assert.IsType<CountersRemovedAnswer>(answer);
        Assert.Equal(["5 -1/-1 2", "6 +1/+1 1"], removed.Removals.Select(r => $"{r.Object.Value} {r.Counter} {r.Count}"));
        Assert.Contains("-1/-1 counter from", chooser.Asked[0].Choices[0], StringComparison.Ordinal);
        Assert.Contains("+1/+1 counter from", chooser.Asked[0].Choices[1], StringComparison.Ordinal);
        Assert.Single(chooser.Asked);
    }

    [Fact]
    public void Removes_up_to_the_most_counters_for_an_effect_and_may_stop_short()
    {
        // "Remove up to three counters from target creature" (CR 608.2d): a -1/-1 counter off the opponent's creature, then
        // stopping, the last choice of the second question.
        var chooser = new ScriptedChooser([1], [1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(5, "Walking Ballista", 2, 2, controller: 2)),
            new RemoveCountersPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Total = 3,
                UpTo = true,
                Pools =
                [
                    new CounterPoolMessage { Object = Handle(5), Counter = "+1/+1", Count = 2 },
                    new CounterPoolMessage { Object = Handle(5), Counter = "-1/-1", Count = 1 },
                ],
            });

        var removed = Assert.IsType<CountersRemovedAnswer>(answer);
        Assert.Equal(["5 -1/-1 1"], removed.Removals.Select(r => $"{r.Object.Value} {r.Counter} {r.Count}"));
        Assert.Contains("up to 3 counters", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.Contains("an opponent's", chooser.Asked[0].Choices[0], StringComparison.Ordinal);
        Assert.Equal("Remove no counters", chooser.Asked[0].Choices[^1]);
        Assert.Equal("Remove no more counters", chooser.Asked[1].Choices[^1]);
        Assert.True(chooser.Spent);
    }

    [Fact]
    public void Chooses_as_many_modes_as_the_spell_asks_for_and_no_repeats()
    {
        var chooser = new ScriptedChooser([1], [0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(3, "Cryptic Command", "{1}{U}{U}{U}")),
            new ChooseModesPromptMessage
            {
                Player = Me,
                Source = Handle(3),
                Modes = ["Counter target spell.", "Return target permanent to its owner's hand.", "Tap all creatures.", "Draw a card."],
                Eligible = [0, 1, 3],
                Count = 2,
            });

        var modes = Assert.IsType<ModesChosenAnswer>(answer);
        Assert.Equal([1, 0], modes.Modes);
        Assert.Equal(2, chooser.Asked.Count);

        // The second question no longer offers the mode already taken.
        Assert.Equal(2, chooser.Asked[1].Choices.Count);
    }

    [Fact]
    public void Discards_exactly_as_many_cards_as_it_must()
    {
        var chooser = new ScriptedChooser([0, 2]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(1, "Mountain"), Card(2, "Lightning Bolt"), Card(3, "Shock"), Card(4, "Forest")),
            new DiscardPromptMessage
            {
                Player = Me,
                Count = 2,
                MaximumHandSize = 7,
                Hand = [Handle(1), Handle(2), Handle(3), Handle(4)],
            });

        Assert.Equal([Handle(1), Handle(3)], Assert.IsType<CardsChosenAnswer>(answer).Cards);
    }

    [Fact]
    public void May_find_nothing_in_a_search_that_allows_it()
    {
        var chooser = new ScriptedChooser([[]]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(1, "Mountain"), Card(2, "Forest")),
            new SearchLibraryPromptMessage
            {
                Player = Me,
                Count = 1,
                CountKind = CountKindView.UpTo,
                Library = [Handle(1), Handle(2)],
                Verdict = new() { Addable = [Handle(1), Handle(2)] },
                Destination = SearchDestinationView.Hand,
            });

        Assert.Empty(Assert.IsType<CardsChosenAnswer>(answer).Cards);
        Assert.True(chooser.Asked[0].UpTo);
    }

    [Fact]
    public void Asks_a_payment_in_the_cards_words_with_its_whole_cost_and_pays_it_when_told()
    {
        // Zoraline, Cosmos Caller: "you may pay {W}{B} and 2 life. When you do, return target nonland permanent card ...".
        var chooser = new ScriptedChooser([[0]]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(1, "Zoraline, Cosmos Caller")),
            new UnlessPaysPromptMessage
            {
                Player = Me,
                Source = Handle(1),
                Subject = Handle(1),
                Verb = "ReturnPermanentCard",
                Question = "pay {cost} to return a nonland permanent card with mana value 3 or less from your graveyard to the battlefield?",
                Detail = "It returns with a finality counter on it",
                No = "Don't pay",
                Total = "{W}{B}",
                Remaining = "{0}",
                Life = 2,
                Options = [new() { Kind = ActionKindView.PayCost }, new() { Kind = ActionKindView.DeclineCost }],
            });

        Assert.Equal(ActionKindView.PayCost, Assert.IsType<TakeActionAnswer>(answer).Kind);
        string heading = chooser.Asked[0].Heading;
        Assert.Contains("pay {W}{B} and 2 life to return a nonland permanent card", heading, StringComparison.Ordinal);
        Assert.Contains("finality counter", heading, StringComparison.Ordinal);
        Assert.Contains("Declining: Don't pay", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void Puts_the_trigger_chosen_to_resolve_first_on_the_stack_last()
    {
        // The last ability named goes on the stack last and therefore resolves first (CR 603.3b), so the question is
        // asked the way a player thinks about it and turned round on the way out.
        var chooser = new ScriptedChooser([1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(1, "Soul Warden", 1, 1), Creature(2, "Blood Artist", 0, 1)),
            new OrderTriggersPromptMessage
            {
                Player = Me,
                Sources = [Handle(1), Handle(2)],
                Texts = ["You gain 1 life.", "Each opponent loses 1 life."],
            });

        Assert.Equal([0, 1], Assert.IsType<TriggersOrderedAnswer>(answer).Order);
    }

    [Fact]
    public void Places_each_card_of_a_scry_on_its_own()
    {
        var chooser = new ScriptedChooser([0], [1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Card(1, "Mountain"), Card(2, "Black Lotus")),
            new DistributeCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Cards = [Handle(1), Handle(2)],
                Destinations =
                [
                    new() { Kind = CardDestinationKindView.LibraryTop, Owner = Me, Max = 2, Eligible = [Handle(1), Handle(2)] },
                    new() { Kind = CardDestinationKindView.LibraryBottom, Owner = Me, Max = 2, Eligible = [Handle(1), Handle(2)] },
                ],
            });

        var placed = Assert.IsType<CardsDistributedAnswer>(answer);
        Assert.Equal([Handle(1)], placed.Placements[0]);
        Assert.Equal([Handle(2)], placed.Placements[1]);
        Assert.Equal(2, chooser.Asked.Count);
    }

    [Fact]
    public void Asks_a_may_as_a_yes_or_no()
    {
        var chooser = new ScriptedChooser([0]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(Creature(4, "Path to Exile", 0, 0, Them)),
            new MayPromptMessage { Player = Me, Source = Handle(4), Verb = "SearchLibrary" });

        Assert.True(Assert.IsType<MayChosenAnswer>(answer).Accept);
        Assert.Equal(2, chooser.Asked[0].Choices.Count);
        Assert.Contains("search library", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    // --- Giving up ---------------------------------------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(EveryPrompt))]
    public void Falls_back_to_the_prompts_own_answer_when_nothing_can_be_got(PromptPayload prompt)
    {
        // A model that will not name a letter must never stall a game. The prompt's own answer is always legal, so a
        // seat that cannot decide plays badly rather than not at all — and the exception is what tells the caller to
        // use it, rather than half a decision being assembled out of one real choice and one invented one.
        var decisions = new Decisions(new SilentChooser());
        GameClient client = Seated(
            Card(2, "Lightning Bolt", "{R}"),
            Creature(5, "Grizzly Bears", 2, 2),
            Creature(6, "Serra Angel", 4, 4, Them));

        try
        {
            Assert.NotNull(decisions.Answer(client, prompt));
        }
        catch (NoChoiceException)
        {
            // Which is what LlmPlay catches to send the default. Either way, nothing else escapes.
        }
    }

    public static TheoryData<PromptPayload> EveryPrompt() =>
    [
        new ChooseNumberPromptMessage { Player = Me, Least = 300, Most = uint.MaxValue },
        new PriorityPromptMessage
        {
            Player = Me,
            Options = [new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true }],
        },
        new ChooseTargetPromptMessage
        {
            Player = Me,
            Ability = Handle(9),
            Candidates = [new() { Object = Handle(5) }, new() { Object = Handle(6) }],
        },
        new ChooseDividedTargetsPromptMessage
        {
            Player = Me,
            Ability = Handle(9),
            Total = 3,
            Candidates = [new() { Object = Handle(5) }, new() { Object = Handle(6) }],
        },
        new DeclareAttackersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(5)],
            AttackablePlayers = [Them],
        },
        new DeclareBlockersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(5)],
            Attackers = [Handle(6)],
            Allowed = [new() { Blocker = Handle(5), Attacker = Handle(6) }],
            MinimumBlockers = [1],
        },
        new ChooseModesPromptMessage
        {
            Player = Me,
            Source = Handle(2),
            Modes = ["Draw a card.", "Gain 3 life."],
            Eligible = [0, 1],
            Count = 1,
        },
        new DiscardPromptMessage { Player = Me, Count = 1, MaximumHandSize = 7, Hand = [Handle(2), Handle(5)] },
        new ChoosePlayerPromptMessage { Player = Me, Source = Handle(2), Candidates = [Me, Them] },
        new ChooseColorsPromptMessage { Player = Me, Source = Handle(2), Choices = [ColorView.Red, ColorView.Blue] },
        new MayPromptMessage { Player = Me, Source = Handle(2), Verb = "PayLife", Amount = 2 },
        new ChooseReplacementPromptMessage
        {
            Player = Me,
            Event = "a creature would die",
            Sources = [Handle(5), Handle(6)],
            Hints = ["exile it instead", "return it to hand"],
        },
        new OrderTriggersPromptMessage
        {
            Player = Me,
            Sources = [Handle(5), Handle(6)],
            Texts = ["gain a life", "draw a card"],
        },
    ];
}
