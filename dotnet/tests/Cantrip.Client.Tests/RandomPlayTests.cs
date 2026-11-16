using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// What the randomized seat may and may not answer.
/// <para>
/// A bot that draws its choices cannot be tested by asserting what it does once, so these do one of two things:
/// draw many times from one prompt and assert an invariant that has to hold every time — every block was one the
/// prompt allowed, no mana ability was ever activated at priority — or draw many times and assert that a choice
/// which is supposed to be reachable is actually reached. The second kind is the one that catches a bot that has
/// quietly stopped exercising something.
/// </para>
/// <para>
/// Every seed here is fixed, so a failure is the same failure on every machine. The bot is random; its tests are not.
/// </para>
/// </summary>
public sealed class RandomPlayTests
{
    private const int Me = 1;
    private const int Them = 2;

    /// <summary>Enough draws that a choice taken one time in three would have to show up.</summary>
    private const int Draws = 400;

    [Fact]
    public void An_x_affordable_up_to_the_signed_limit_is_stepped_to_no_further_than_a_seat_steps()
    {
        var prompt = new PayCostsPromptMessage { AnnouncesXOrKicks = true, HasX = true, X = AutoPlay.MostStepped,
            MostAffordable = int.MaxValue, MostAnnounced = int.MaxValue };
        prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.IncreaseX });
        prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.DecreaseX });
        prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting });
        var answer = Assert.IsType<TakeActionAnswer>(new RandomPlay(seed: 7).Decide(prompt));
        Assert.NotEqual(ActionKindView.IncreaseX, answer.Kind);
    }

    // --- Priority ---------------------------------------------------------------------------------------------------

    [Fact]
    public void Never_activates_a_mana_ability_at_priority()
    {
        // Mana made at priority is gone when the step ends and the land stays tapped, so a seat that tapped for mana
        // nobody had asked for would spend its turn on nothing. It is made while paying instead.
        PriorityPromptMessage prompt = Priority(
            Mana(source: 1),
            AnyColorMana(source: 2),
            EachColorMana(source: 3),
            Ability(source: 4));

        var bot = new RandomPlay(seed: 7);
        var taken = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            if (bot.Decide(prompt) is TakeActionAnswer action)
            {
                taken.Add(action.Source.Value);
            }
        }

        Assert.Equal([4], taken);
    }

    [Fact]
    public void Passes_when_the_only_thing_offered_is_mana()
    {
        PriorityPromptMessage prompt = Priority(Mana(source: 1), AnyColorMana(source: 2));
        var bot = new RandomPlay(seed: 7);

        for (int i = 0; i < 20; i++)
        {
            Assert.IsType<PassAnswer>(bot.Decide(prompt));
        }
    }

    [Fact]
    public void Passing_is_one_of_the_draws_rather_than_a_last_resort()
    {
        // A seat that acted whenever it could would never hold a trick back, and would never let an end step arrive
        // with anything up (CR 117.3d).
        PriorityPromptMessage prompt = Priority(Ability(source: 4));
        var bot = new RandomPlay(seed: 7);

        int passes = 0;
        int actions = 0;

        for (int i = 0; i < Draws; i++)
        {
            if (bot.Decide(prompt) is PassAnswer) passes++; else actions++;
        }

        Assert.True(passes > 0, "never passed");
        Assert.True(actions > 0, "never acted");
    }

    [Fact]
    public void Leaves_alone_a_spell_or_ability_the_server_says_it_cannot_pay_for()
    {
        // Announcing it, failing to pay and giving it up (CR 733.1) puts the seat back where it was with the same
        // action offered, which is a loop and not a decision.
        PriorityPromptMessage prompt = Priority(
            new ActionOptionMessage { Kind = ActionKindView.CastSpell, Source = Handle(1), Affordable = false },
            new ActionOptionMessage { Kind = ActionKindView.ActivateAbility, Source = Handle(2), Affordable = false },
            new ActionOptionMessage { Kind = ActionKindView.Plot, Source = Handle(3), Affordable = false });

        var bot = new RandomPlay(seed: 7);

        for (int i = 0; i < 20; i++)
        {
            Assert.IsType<PassAnswer>(bot.Decide(prompt));
        }
    }

    [Fact]
    public void Plays_a_land_although_nothing_says_a_land_is_affordable()
    {
        // A land has no cost, so the server sets nothing about affording it; a seat that demanded it would never
        // play one and would never get past its first turn.
        PriorityPromptMessage prompt = Priority(
            new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = Handle(1) });

        var bot = new RandomPlay(seed: 7);
        bool played = false;

        for (int i = 0; i < Draws && !played; i++)
        {
            played = bot.Decide(prompt) is TakeActionAnswer { Kind: ActionKindView.PlayLand };
        }

        Assert.True(played, "never played a land");
    }

    [Fact]
    public void Aims_an_action_only_at_the_candidates_it_was_offered_and_sometimes_at_nothing()
    {
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.CastSpell,
            Source = Handle(1),
            Affordable = true,
            TargetsRequired = 1,
            TargetsOptional = true,
            TargetCandidates = [Target(2), Target(3)],
        };

        var bot = new RandomPlay(seed: 11);
        var aimed = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            if (bot.Decide(Priority(option)) is not TakeActionAnswer action)
            {
                continue;
            }

            // "Up to one target" may be aimed at nothing (CR 601.2c); -1 stands for that here.
            aimed.Add(action.Targets.Count == 0 ? -1 : action.Targets[0].Object.Value);
            Assert.True(action.Targets.Count <= 1);
        }

        Assert.Equal([-1, 2, 3], aimed.OrderBy(value => value));
    }

    // --- Combat ------------------------------------------------------------------------------------------------------

    [Fact]
    public void Gives_every_creature_its_own_choice_of_staying_home_a_seat_or_a_planeswalker()
    {
        var prompt = new DeclareAttackersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(1)],
            AttackablePlayers = [Them],
            AttackablePlaneswalkers = [Handle(9)],
        };

        var bot = new RandomPlay(seed: 3);
        var seen = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var declared = Assert.IsType<AttackersDeclaredAnswer>(bot.Decide(prompt));
            seen.Add(declared.Attacks.Count == 0
                ? "home"
                : declared.Attacks[0].TargetPermanent.IsNone ? $"seat {declared.Attacks[0].TargetPlayer}" : "planeswalker");
        }

        Assert.Equal(["home", "planeswalker", "seat 2"], seen.OrderBy(what => what));
    }

    [Fact]
    public void Attacks_with_a_creature_that_must_attack_every_time()
    {
        DeclareAttackersPromptMessage prompt = FixtureTransport.JudgeAttackers(new DeclareAttackersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(1), Handle(2)],
            AttackablePlayers = [Them],
            Requirements = [new AttackRequirementMessage { Attacker = Handle(1) }],
        });

        var bot = new RandomPlay(seed: 3);
        bool stayedHome = false;

        for (int i = 0; i < Draws; i++)
        {
            var declared = Assert.IsType<AttackersDeclaredAnswer>(bot.Decide(prompt));
            Assert.Contains(declared.Attacks, attack => attack.Attacker == Handle(1));
            stayedHome |= declared.Attacks.All(attack => attack.Attacker != Handle(2));
        }

        // The one without a requirement still gets its own choice.
        Assert.True(stayedHome, "the unrequired creature always attacked");
    }

    [Fact]
    public void Sends_a_goaded_creature_only_where_its_requirement_allows()
    {
        // CR 701.15b: goaded by Them, so it attacks a player other than Them — which rules out the planeswalker as well,
        // because attacking one obeys one requirement where attacking seat 3 obeys two.
        DeclareAttackersPromptMessage prompt = FixtureTransport.JudgeAttackers(new DeclareAttackersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(1)],
            AttackablePlayers = [Them, 3],
            AttackablePlaneswalkers = [Handle(9)],
            Requirements =
            [
                new AttackRequirementMessage { Attacker = Handle(1) },
                new AttackRequirementMessage { Attacker = Handle(1), OtherThan = Them },
            ],
        });

        var bot = new RandomPlay(seed: 3);

        for (int i = 0; i < Draws; i++)
        {
            AttackMessage attack = Assert.Single(Assert.IsType<AttackersDeclaredAnswer>(bot.Decide(prompt)).Attacks);
            Assert.True(attack.TargetPermanent.IsNone);
            Assert.Equal(3, attack.TargetPlayer);
            Assert.True(FixtureTransport.StandInAttackVerdict(prompt, [attack]).Complete);
        }
    }

    [Fact]
    public void Never_sends_a_creature_at_a_seat_it_is_forbidden_to_attack_and_must_attacks_go_elsewhere()
    {
        // CR 508.1c: "you can't be attacked except by creatures with flying" closes the seat to creature 1 alone.
        DeclareAttackersPromptMessage prompt = FixtureTransport.JudgeAttackers(new DeclareAttackersPromptMessage
        {
            Player = Me,
            Eligible = [Handle(1), Handle(2)],
            AttackablePlayers = [Them],
            AttackablePlaneswalkers = [Handle(9)],
            Requirements = [new AttackRequirementMessage { Attacker = Handle(1) }],
            Forbidden = [new AttackMessage { Attacker = Handle(1), TargetPlayer = Them }],
        });

        var bot = new RandomPlay(seed: 3);

        for (int i = 0; i < Draws; i++)
        {
            var declared = Assert.IsType<AttackersDeclaredAnswer>(bot.Decide(prompt));
            Assert.DoesNotContain(declared.Attacks, prompt.IsForbidden);
        }

        AttackMessage fallback = Assert.Single(Assert.IsType<AttackersDeclaredAnswer>(prompt.Default()).Attacks);
        Assert.Equal(Handle(9), fallback.TargetPermanent);
        Assert.Equal([Handle(9)], prompt.AttacksFor(Handle(1)).Select(attack => attack.TargetPermanent));
    }

    [Fact]
    public void Declares_only_blocks_the_prompt_allows_and_uses_each_blocker_once()
    {
        // Flying (CR 702.9b) is why the allowed list is narrower than every blocker against every attacker: the
        // second blocker may only block the ground attacker.
        var prompt = new DeclareBlockersPromptMessage
        {
            Player = Me,
            Attackers = [Handle(1), Handle(2)],
            Eligible = [Handle(3), Handle(4)],
            MinimumBlockers = [1, 1],
            Allowed =
            [
                Pair(3, 1), Pair(3, 2), Pair(4, 2),
            ],
        };

        var bot = new RandomPlay(seed: 5);
        var seen = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var declared = Assert.IsType<BlockersDeclaredAnswer>(bot.Decide(prompt));

            foreach (BlockMessage block in declared.Blocks)
            {
                Assert.Contains(prompt.Allowed, allowed =>
                    allowed.Blocker == block.Blocker && allowed.Attacker == block.Attacker);
            }

            Assert.Equal(
                declared.Blocks.Select(block => block.Blocker).Distinct().Count(),
                declared.Blocks.Count);

            seen.Add(string.Join(",", declared.Blocks.Select(block => $"{block.Blocker.Value}>{block.Attacker.Value}")));
        }

        // Every combination, gang block included, is reachable — the gang is what asks the attacker for a damage
        // assignment order (CR 509.2), and a seat that never made one could not reach that screen.
        Assert.Contains("", seen);
        Assert.Contains("3>2,4>2", seen);
    }

    [Fact]
    public void Takes_back_blocks_that_fall_short_of_an_attackers_minimum()
    {
        // Menace and "except by three or more" are stated per attacker, and a declaration one blocker short is
        // refused whole (CR 509.1b) — which would throw away the blocks on every other attacker with it.
        var prompt = new DeclareBlockersPromptMessage
        {
            Player = Me,
            Attackers = [Handle(1)],
            Eligible = [Handle(3), Handle(4), Handle(5)],
            MinimumBlockers = [2],
            Allowed = [Pair(3, 1), Pair(4, 1), Pair(5, 1)],
        };

        var bot = new RandomPlay(seed: 5);
        var counts = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            int blocking = Assert.IsType<BlockersDeclaredAnswer>(bot.Decide(prompt)).Blocks.Count;
            Assert.True(blocking is 0 or >= 2, $"declared {blocking} blocker(s) against a minimum of 2");
            counts.Add(blocking);
        }

        Assert.Contains(0, counts);
        Assert.Contains(2, counts);
    }

    [Fact]
    public void Leaves_out_blockers_past_an_attackers_maximum()
    {
        // "Can't be blocked by more than one creature" (CR 509.1b): a declaration of two on it would be refused whole.
        var prompt = new DeclareBlockersPromptMessage
        {
            Player = Me,
            Attackers = [Handle(1)],
            Eligible = [Handle(3), Handle(4), Handle(5)],
            MinimumBlockers = [1],
            MaximumBlockers = [1],
            Allowed = [Pair(3, 1), Pair(4, 1), Pair(5, 1)],
        };

        var bot = new RandomPlay(seed: 5);
        var counts = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            int blocking = Assert.IsType<BlockersDeclaredAnswer>(bot.Decide(prompt)).Blocks.Count;
            Assert.True(blocking is 0 or 1, $"declared {blocking} blockers against a maximum of 1");
            counts.Add(blocking);
        }

        Assert.Contains(0, counts);
        Assert.Contains(1, counts);
    }

    // --- Choosing ----------------------------------------------------------------------------------------------------

    [Fact]
    public void Chooses_targets_from_the_candidates_without_favouring_an_opponents()
    {
        // A seat that only ever aimed at an opponent's things never exercises a card aimed at its own, and half of
        // what a removal spell can be pointed at is its controller's.
        var prompt = new ChooseTargetPromptMessage { Player = Me, Candidates = [Target(1), Target(2), Target(3)] };
        var bot = new RandomPlay(seed: 13);
        var aimed = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            TargetMessage chosen = Assert.Single(Assert.IsType<TargetsChosenAnswer>(bot.Decide(prompt)).Targets);
            aimed.Add(chosen.Object.Value);
        }

        Assert.Equal([1, 2, 3], aimed.OrderBy(value => value));
    }

    [Fact]
    public void Divides_a_total_so_every_chosen_target_gets_some_of_it()
    {
        // CR 601.2d: a target chosen for a divided slot has to be given at least one, so no more of them may be
        // chosen than the total covers.
        var prompt = new ChooseDividedTargetsPromptMessage
        {
            Player = Me,
            Candidates = [Target(1), Target(2), Target(3)],
            Total = 2,
        };

        var bot = new RandomPlay(seed: 17);
        var spreads = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            var divided = Assert.IsType<TargetsDividedAnswer>(bot.Decide(prompt));

            Assert.Equal(2, divided.Shares.Sum(share => share.Amount));
            Assert.All(divided.Shares, share => Assert.True(share.Amount >= 1));
            Assert.Equal(divided.Shares.Select(share => share.Target.Object).Distinct().Count(), divided.Shares.Count);
            spreads.Add(divided.Shares.Count);
        }

        Assert.Equal([1, 2], spreads.OrderBy(count => count));
    }

    [Fact]
    public void Chooses_as_many_cards_as_a_choice_asks_for_and_up_to_that_many_where_fewer_will_do()
    {
        var exact = new ChooseCardsPromptMessage { Player = Me, Cards = [Handle(1), Handle(2), Handle(3)],
            Verdict = new() { Addable = [Handle(1), Handle(2), Handle(3)] }, Count = 2 };
        var upTo = new ChooseCardsPromptMessage { Player = Me, Cards = [Handle(1), Handle(2), Handle(3)],
            Verdict = new() { Addable = [Handle(1), Handle(2), Handle(3)] }, Count = 2, CountKind = CountKindView.UpTo };

        var bot = new RandomPlay(seed: 19);
        var counts = new HashSet<int>();

        for (int i = 0; i < Draws; i++)
        {
            var chosen = Assert.IsType<CardsChosenAnswer>(bot.Decide(exact));
            Assert.Equal(2, chosen.Cards.Count);
            Assert.All(chosen.Cards, card => Assert.Contains(card, exact.Verdict.Addable));
            Assert.Equal(chosen.Cards.Distinct().Count(), chosen.Cards.Count);

            counts.Add(Assert.IsType<CardsChosenAnswer>(bot.Decide(upTo)).Cards.Count);
        }

        Assert.Equal([0, 1, 2], counts.OrderBy(count => count));
    }

    [Fact]
    public void Meets_a_total_power_a_choice_asks_for_while_still_drawing_which_cards()
    {
        // Saddle (CR 702.171a): which creatures saddle is a real choice, and the answer still has to add up.
        var prompt = new ChooseCardsPromptMessage
        {
            Player = Me,
            Cards = [Handle(1), Handle(2), Handle(3)],
            Verdict = new() { Addable = [Handle(1), Handle(2), Handle(3)] },
            Count = 3,
            CountKind = CountKindView.UpTo,
            MinimumPower = 3,
            Powers = [1, 2, 3],
        };

        var bot = new RandomPlay(seed: 23);
        var sets = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var chosen = Assert.IsType<CardsChosenAnswer>(bot.Decide(prompt));
            int power = chosen.Cards.Sum(card => prompt.Powers[prompt.Cards.IndexOf(card)]);

            Assert.True(power >= 3, $"chose {power} power against a minimum of 3");
            sets.Add(string.Join(",", chosen.Cards.Select(card => card.Value).OrderBy(value => value)));
        }

        Assert.True(sets.Count > 1, "always saddled with the same creatures");
    }

    [Fact]
    public void Never_taps_a_creature_whose_negative_power_would_take_from_the_total()
    {
        // Crew 3 with a 3/3 and a -2/1 (CR 107.1b: a negative power is added as it is, so the two total 1): only the 3/3
        // meets it, and the default answer takes every creature with positive power.
        var prompt = new ChooseCardsPromptMessage
        {
            Player = Me,
            Cards = [Handle(1), Handle(2)],
            Verdict = new() { Addable = [Handle(1), Handle(2)] },
            Count = 2,
            CountKind = CountKindView.UpTo,
            MinimumPower = 3,
            Powers = [-2, 3],
        };

        var bot = new RandomPlay(seed: 29);

        for (int i = 0; i < Draws; i++)
        {
            var chosen = Assert.IsType<CardsChosenAnswer>(bot.Decide(prompt));
            Assert.Equal([Handle(2)], chosen.Cards);
        }

        Assert.Equal([Handle(2)], Assert.IsType<CardsChosenAnswer>(prompt.Default()).Cards);
    }

    [Fact]
    public void Meets_a_total_mana_value_a_choice_asks_for_while_still_drawing_which_cards()
    {
        // Collect evidence (CR 701.59a): which cards is a real choice, and the answer still has to add up.
        var prompt = new ChooseCardsPromptMessage
        {
            Player = Me,
            Cards = [Handle(1), Handle(2), Handle(3), Handle(4)],
            Verdict = new() { Addable = [Handle(1), Handle(2), Handle(3), Handle(4)] },
            Count = 4,
            CountKind = CountKindView.UpTo,
            MinimumManaValue = 6,
            ManaValues = [3, 0, 4, 2],
        };

        var bot = new RandomPlay(seed: 31);
        var sets = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var chosen = Assert.IsType<CardsChosenAnswer>(bot.Decide(prompt));
            int total = chosen.Cards.Sum(card => prompt.ManaValues[prompt.Cards.IndexOf(card)]);

            Assert.True(total >= 6, $"chose {total} mana value against a minimum of 6");
            sets.Add(string.Join(",", chosen.Cards.Select(card => card.Value).OrderBy(value => value)));
        }

        Assert.True(sets.Count > 1, "always collected the same evidence");
    }

    [Fact]
    public void Stays_within_a_total_mana_value_a_choice_limits_while_still_drawing_which_cards()
    {
        // "Up to two creature cards with total mana value 6 or less" (CR 202.3): never over it, never more than two.
        var prompt = new ChooseCardsPromptMessage
        {
            Player = Me,
            Cards = [Handle(1), Handle(2), Handle(3), Handle(4)],
            Verdict = new() { Addable = [Handle(1), Handle(2), Handle(3), Handle(4)] },
            Count = 2,
            CountKind = CountKindView.UpTo,
            ManaValues = [4, 3, 2, 6],
            MaximumTotalManaValue = 6,
        };

        var bot = new RandomPlay(seed: 31);
        var sets = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var chosen = Assert.IsType<CardsChosenAnswer>(bot.Decide(prompt));
            int total = chosen.Cards.Sum(card => prompt.ManaValues[prompt.Cards.IndexOf(card)]);

            Assert.True(total <= 6, $"chose {total} mana value against a most of 6");
            Assert.True(chosen.Cards.Count <= 2, $"chose {chosen.Cards.Count} cards of up to 2");
            sets.Add(string.Join(",", chosen.Cards.Select(card => card.Value).OrderBy(value => value)));
        }

        Assert.True(sets.Count > 1, "always chose the same cards");
    }

    [Fact]
    public void Orders_triggers_every_way_round()
    {
        // CR 603.3b: any order is legal and the order decides which resolves first, so this is a real difference
        // between two runs rather than a formality.
        var prompt = new OrderTriggersPromptMessage { Player = Me, Sources = [Handle(1), Handle(2)], Texts = ["a", "b"] };
        var bot = new RandomPlay(seed: 29);
        var orders = new HashSet<string>();

        for (int i = 0; i < Draws; i++)
        {
            var ordered = Assert.IsType<TriggersOrderedAnswer>(bot.Decide(prompt));
            Assert.Equal([0, 1], ordered.Order.OrderBy(index => index));
            orders.Add(string.Join(",", ordered.Order));
        }

        Assert.Equal(["0,1", "1,0"], orders.OrderBy(order => order));
    }

    [Fact]
    public void Answers_a_may_both_ways()
    {
        // Which is what lets a chain of them end — Chain of Smog's copy, Lim-Dûl's Vault's look again — without
        // naming either card.
        var prompt = new MayPromptMessage { Player = Me, Verb = "CopySpell" };
        var bot = new RandomPlay(seed: 31);
        var answers = new HashSet<bool>();

        for (int i = 0; i < Draws; i++)
        {
            answers.Add(Assert.IsType<MayChosenAnswer>(bot.Decide(prompt)).Accept);
        }

        Assert.Equal(2, answers.Count);
    }

    // --- X -----------------------------------------------------------------------------------------------------------

    [Fact]
    public void Casts_a_spell_with_X_only_once_casting_is_offered_and_for_a_value_that_varies()
    {
        // The server offers the raise only while a larger X could be paid for and offers casting only once nothing
        // is owed — the highlighted "okay" — so the value needs no arithmetic here to be legal.
        var chosen = new HashSet<int>();

        for (int seed = 0; seed < 12; seed++)
        {
            var bot = new RandomPlay(seed);

            bot.Decide(Priority(new ActionOptionMessage
            {
                Kind = ActionKindView.CastSpell,
                Source = Handle(1),
                Affordable = true,
                HasX = true,
                MaxX = 3,
            }));

            chosen.Add(PayUntilCast(bot, largestX: 3));
        }

        Assert.True(chosen.Count > 1, $"always cast it for the same X: {string.Join(", ", chosen)}");
        Assert.All(chosen, x => Assert.InRange(x, 0, 3));
    }

    [Fact]
    public void Chooses_how_often_to_pay_a_kicker_although_nothing_offered_a_ceiling_for_it()
    {
        // A kicker or a buyback is X in the same prompt but not {X} in the mana cost (CR 702.33), so the announcement
        // said nothing about how large it could be and the draw is made while paying.
        var chosen = new HashSet<int>();

        for (int seed = 0; seed < 12; seed++)
        {
            var bot = new RandomPlay(seed);
            bot.Decide(Priority(new ActionOptionMessage { Kind = ActionKindView.CastSpell, Source = Handle(1), Affordable = true }));
            chosen.Add(PayUntilCast(bot, largestX: 2, kicker: "{2}"));
        }

        Assert.True(chosen.Count > 1, $"always kicked it the same number of times: {string.Join(", ", chosen)}");
        Assert.All(chosen, x => Assert.InRange(x, 0, 2));
    }

    [Fact]
    public void Gives_up_on_a_value_of_X_it_cannot_pay_for_rather_than_raising_it_again()
    {
        // The loop this rules out: raise X to the value drawn, find it cannot be paid for, lower it — and then raise
        // it straight back to the same value, for ever, without the game moving and without any answer being
        // refused. A kicker is where it bites, because nothing offered a ceiling to draw the value from.
        foreach (int seed in Enumerable.Range(0, 24))
        {
            var bot = new RandomPlay(seed);
            bot.Decide(Priority(new ActionOptionMessage { Kind = ActionKindView.CastSpell, Source = Handle(1), Affordable = true }));

            int x = 0;
            bool settled = false;

            for (int step = 0; step < 100 && !settled; step++)
            {
                // Raising is offered, and every value above nothing owes {4} that nothing here can pay.
                var prompt = new PayCostsPromptMessage
                {
                    Player = Me,
                    Spell = Handle(1),
                    HasX = true,
                    X = x,
                    Kicker = "{4}",
                    RemainingGeneric = x > 0 ? 4 : 0,
                };

                prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.IncreaseX });
                if (x > 0) prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.DecreaseX });
                if (x == 0) prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting });
                prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.CancelCasting });

                var answer = Assert.IsType<TakeActionAnswer>(bot.Decide(prompt));
                Assert.Contains(prompt.Options, option => option.Kind == answer.Kind);

                switch (answer.Kind)
                {
                    case ActionKindView.IncreaseX: x++; break;
                    case ActionKindView.DecreaseX: x--; break;
                    default: settled = true; break;
                }

                Assert.True(x <= 2, "kept raising an X it had already failed to pay for");
            }

            Assert.True(settled, $"seed {seed} never stopped raising and lowering X");
        }
    }

    /// <summary>
    /// Answers pay-costs prompts until the spell is cast, standing in for the server: X may be raised while it is
    /// under <paramref name="largestX"/> and lowered while it is over zero, nothing is ever owed, and casting is
    /// offered throughout — so the only reason to stop raising is the bot's own choice.
    /// </summary>
    /// <returns>The X it cast for.</returns>
    private static int PayUntilCast(RandomPlay bot, int largestX, string kicker = "")
    {
        int x = 0;

        for (int step = 0; step < 100; step++)
        {
            var prompt = new PayCostsPromptMessage { Player = Me, Spell = Handle(1), HasX = true, X = x, Kicker = kicker };

            if (x < largestX) prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.IncreaseX });
            if (x > 0) prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.DecreaseX });
            prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting });
            prompt.Options.Add(new ActionOptionMessage { Kind = ActionKindView.CancelCasting });

            var answer = Assert.IsType<TakeActionAnswer>(bot.Decide(prompt));

            // Nothing may be answered that was not offered: casting a spell whose "okay" is not lit is the mistake
            // this is here to rule out.
            Assert.Contains(prompt.Options, option => option.Kind == answer.Kind);

            switch (answer.Kind)
            {
                case ActionKindView.IncreaseX: x++; break;
                case ActionKindView.DecreaseX: x--; break;
                case ActionKindView.ConfirmCasting: return x;
                default: Assert.Fail($"gave up on a spell it could pay for: {answer.Kind}"); break;
            }
        }

        Assert.Fail("never settled on an X");
        return -1;
    }

    // --- Giving up ---------------------------------------------------------------------------------------------------

    [Fact]
    public void Falls_back_to_passing_once_the_same_question_has_been_refused_enough_times()
    {
        // The server sends the identical question back when it refuses an answer, which is the only notice the seat
        // gets. Not every rule a declaration is judged by is in the prompt — a creature that must block is not — so
        // drawing again is right up to a point, and then the prompt's own answer is the way out.
        var transport = new Refusing();
        var client = new GameClient(transport);
        var bot = new RandomPlay(seed: 37);

        transport.Inbound.Enqueue(new PromptMessage
        {
            RequestId = 1,
            Prompt = Priority(new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = Handle(1) }),
        });

        var answers = new List<AnswerPayload>();

        for (int i = 0; i < 6; i++)
        {
            client.Pump();
            Assert.True(bot.Act(client), "stopped answering a question that is still outstanding");
            answers.Add(((AnswerMessage)transport.Sent[^1]).Answer);

            // What the server does with a refusal: the same prompt, id and all, goes back out.
            transport.Inbound.Enqueue(new PromptMessage
            {
                RequestId = 1,
                Prompt = Priority(new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = Handle(1) }),
            });
        }

        // Three draws, then the prompt's own default — passing — for good, and it stops claiming it has another
        // answer to try so a harness can report the refusal instead of spinning.
        Assert.False(bot.Retrying);
        Assert.All(answers.Skip(3), answer => Assert.IsType<PassAnswer>(answer));
    }

    [Fact]
    public void Draws_afresh_for_the_next_question_after_giving_up_on_one()
    {
        var transport = new Refusing();
        var client = new GameClient(transport);
        var bot = new RandomPlay(seed: 37);

        for (int id = 1; id <= 6; id++)
        {
            transport.Inbound.Enqueue(new PromptMessage
            {
                RequestId = id,
                Prompt = Priority(new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = Handle(1) }),
            });

            client.Pump();
            Assert.True(bot.Act(client));
            Assert.True(bot.Retrying, "a fresh question was treated as a refused one");
        }
    }

    // --- Building prompts --------------------------------------------------------------------------------------------

    private static ViewHandle Handle(int value) => new(value);

    private static TargetMessage Target(int handle) => new() { Object = Handle(handle) };

    private static BlockMessage Pair(int blocker, int attacker) => new() { Blocker = Handle(blocker), Attacker = Handle(attacker) };

    private static PriorityPromptMessage Priority(params ActionOptionMessage[] options) =>
        new() { Player = Me, Options = [.. options] };

    private static ActionOptionMessage Mana(int source) => new()
    {
        Kind = ActionKindView.ActivateAbility,
        Source = Handle(source),
        Affordable = true,
        Produces = [ManaTypeView.Red],
    };

    private static ActionOptionMessage AnyColorMana(int source) => new()
    {
        Kind = ActionKindView.ActivateAbility,
        Source = Handle(source),
        Affordable = true,
        ProducesOfAnyOneColor = 1,
        ColorChoices = [ColorView.Red],
    };

    private static ActionOptionMessage EachColorMana(int source) => new()
    {
        Kind = ActionKindView.ActivateAbility,
        Source = Handle(source),
        Affordable = true,
        ProducesOfAnyColorEach = 2,
        ColorChoices = [ColorView.Red, ColorView.Green],
    };

    private static ActionOptionMessage Ability(int source) =>
        new() { Kind = ActionKindView.ActivateAbility, Source = Handle(source), Affordable = true };

    /// <summary>A transport that only carries what a test puts in it, so a refusal can be staged exactly.</summary>
    private sealed class Refusing : IGameTransport
    {
        public Queue<ServerMessage> Inbound { get; } = [];

        public List<ClientMessage> Sent { get; } = [];

        public bool IsConnected => true;

        public void Send(ClientMessage message) => Sent.Add(message);

        public IReadOnlyList<ServerMessage> Receive()
        {
            List<ServerMessage> messages = [.. Inbound];
            Inbound.Clear();
            return messages;
        }

        public void Dispose()
        {
        }
    }

    /// <summary>Pawprints (CR 700.2i): drawn modes never add up to more than the most, and none at all is among the draws.</summary>
    [Fact]
    public void Pawprint_modes_drawn_stay_within_the_pawprints()
    {
        var prompt = new ChooseModesPromptMessage
        {
            Player = 0,
            Modes = ["a", "b", "c"],
            Eligible = [0, 1, 2],
            AtMost = 5,
            MayRepeat = true,
            Pawprints = [1, 2, 3],
        };
        var bot = new RandomPlay(seed: 5);
        bool none = false;
        for (int i = 0; i < 200; i++)
        {
            List<int> modes = Assert.IsType<ModesChosenAnswer>(bot.Decide(prompt)).Modes;
            Assert.True(modes.Sum(prompt.Weight) <= 5);
            none |= modes.Count == 0;
        }

        Assert.True(none);
    }
}
