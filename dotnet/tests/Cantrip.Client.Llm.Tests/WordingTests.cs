using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Llm.Tests.Board;

namespace Cantrip.Client.Llm.Tests;

/// <summary>
/// What a model is told: the zone a card is cast from and the way it is cast, where the cards of a choice are and what is done
/// with them, whose library is searched and where the cards go. Each said as the engine says it, not as a purpose it was once
/// read as.
/// </summary>
public sealed class WordingTests
{
    [Fact]
    public void Tapping_a_land_to_pay_does_not_tell_the_model_to_tap_creatures()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView land = In(Card(3, "Forest", "{0}"), ZoneKindView.Battlefield, Me);
        ObjectView other = In(Card(4, "Island", "{0}"), ZoneKindView.Battlefield, Me);
        new Decisions(chooser).Answer(Seated(land, other), new ChooseCardsPromptMessage
        {
            Player = Me, Source = Handle(9), Purpose = CardChoicePurposeView.TapToPay,
            Cards = [land.Handle, other.Handle], Verdict = new() { Addable = [land.Handle, other.Handle] }, Count = 1,
        });

        Assert.Contains("tap eligible permanents", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.DoesNotContain("tap creatures", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    [Fact]
    public void Bands_with_other_preserves_choices_without_ordinary_banding()
    {
        var chooser = new ScriptedChooser([0, 1]);
        ObjectView first = In(Card(3, "Reptil", "{2}{G}"), ZoneKindView.Battlefield, Me);
        ObjectView second = In(Card(4, "Rick Jones", "{W}"), ZoneKindView.Battlefield, Me);
        var answer = Assert.IsType<CardsChosenAnswer>(new Decisions(chooser).Answer(Seated(first, second), new ChooseCardsPromptMessage
        {
            Player = Me, Source = Handle(9), Purpose = CardChoicePurposeView.Band,
            Cards = [first.Handle, second.Handle], WithoutBanding = [first.Handle, second.Handle],
            Verdict = new() { Addable = [first.Handle, second.Handle], Complete = true },
            Count = 2, CountKind = CountKindView.UpTo,
        }));

        Assert.Equal(new[] { first.Handle, second.Handle }, answer.Cards);
        Assert.Contains("bands with other", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.DoesNotContain("has banding", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    [Fact]
    public void Restricted_untapping_does_not_ask_the_model_for_an_exact_candidate_count()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView artifact = In(Card(3, "Sol Ring", "{1}"), ZoneKindView.Battlefield, Me);
        ObjectView second = In(Card(4, "Mox Jet", "{0}"), ZoneKindView.Battlefield, Me);
        ObjectView third = In(Card(5, "Mox Pearl", "{0}"), ZoneKindView.Battlefield, Me);
        ObjectView fourth = In(Card(6, "Mox Ruby", "{0}"), ZoneKindView.Battlefield, Me);
        new Decisions(chooser).Answer(Seated(artifact, second, third, fourth), new ChooseCardsPromptMessage
        {
            Player = Me, Source = Handle(9), Purpose = CardChoicePurposeView.Untap,
            Cards = [artifact.Handle, second.Handle, third.Handle, fourth.Handle],
            Verdict = new() { Addable = [artifact.Handle, second.Handle, third.Handle, fourth.Handle] },
            Count = 3, CountKind = CountKindView.AsManyAsCan, Kind = "permanent to untap",
        });

        Assert.Contains("untap restrictions allow", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.DoesNotContain("untap 3", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    private static ObjectView In(ObjectView card, ZoneKindView zone, int owner)
    {
        card.Zone = zone;
        card.ZoneOwner = owner;
        return card;
    }

    [Fact]
    public void A_flashback_cast_names_the_graveyard_and_the_cost_the_engine_gives_it()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView looting = In(Card(2, "Faithless Looting", "{R}", "Draw two cards."), ZoneKindView.Graveyard, Me);

        new Decisions(chooser).Answer(
            Seated(looting),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true, AbilityIndex = 0, Description = "Flashback {R}" },
                ],
            });

        string row = chooser.Asked[0].Choices[0];
        Assert.Contains("from your graveyard", row, StringComparison.Ordinal);
        Assert.Contains("(Flashback {R})", row, StringComparison.Ordinal);
    }

    [Fact]
    public void An_alternative_cost_is_named_by_its_own_text()
    {
        var chooser = new ScriptedChooser([0]);

        new Decisions(chooser).Answer(
            Seated(Card(2, "Reanimate", "{B}", "Put target creature card from a graveyard onto the battlefield.")),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true, AbilityIndex = 1, Description = "Flashback {2}{R}, Discard a card" },
                ],
            });

        Assert.Contains("by an alternative cost (Flashback {2}{R}, Discard a card)", chooser.Asked[0].Choices[0], StringComparison.Ordinal);
    }

    [Fact]
    public void An_exile_says_where_the_cards_are_not_that_they_are_in_a_hand()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView elves = Creature(3, "Llanowar Elves", 1, 1, Them);
        ObjectView ornithopter = In(Card(4, "Ornithopter", "{0}"), ZoneKindView.Graveyard, Them);

        new Decisions(chooser).Answer(
            Seated(elves, ornithopter),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.Exile,
                Cards = [Handle(3), Handle(4)],
                Verdict = new() { Addable = [Handle(3), Handle(4)] },
                Count = 1,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("the battlefield", heading, StringComparison.Ordinal);
        Assert.Contains("Opponent's graveyard", heading, StringComparison.Ordinal);
        Assert.DoesNotContain("hand", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_choice_of_a_card_to_play_does_not_say_it_is_free()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView bolt = In(Card(3, "Lightning Bolt", "{R}"), ZoneKindView.Exile, Me);
        ObjectView island = In(Card(4, "Island", "{0}"), ZoneKindView.Exile, Me);

        new Decisions(chooser).Answer(
            Seated(bolt, island),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.Play,
                Cards = [Handle(3), Handle(4)],
                Verdict = new() { Addable = [Handle(3), Handle(4)] },
                Count = 1,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("you may play this turn", heading, StringComparison.Ordinal);
        Assert.DoesNotContain("without paying", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_free_cast_of_copies_says_copies_free_and_what_is_left_of_the_budget()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView giant = In(Card(3, "Bonecrusher Giant", "{2}{R}"), ZoneKindView.Exile, Me);

        new Decisions(chooser).Answer(
            Seated(giant),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.CastCopyFreeWithin,
                Cards = [Handle(3), Handle(4)],
                Verdict = new() { Addable = [Handle(3)] },
                Count = 1,
                CountKind = CountKindView.UpTo,
                MaximumManaValue = 4,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("copy", heading, StringComparison.Ordinal);
        Assert.Contains("from exile", heading, StringComparison.Ordinal);
        Assert.Contains("without paying", heading, StringComparison.Ordinal);
        Assert.Contains("at most 4", heading, StringComparison.Ordinal);
        Assert.DoesNotContain("graveyard", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_free_cast_of_copies_limited_by_count_says_no_mana_value_budget()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView giant = In(Card(3, "Bonecrusher Giant", "{2}{R}"), ZoneKindView.Exile, Me);

        new Decisions(chooser).Answer(
            Seated(giant),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.CastCopyFree,
                Cards = [Handle(3), Handle(4)],
                Verdict = new() { Addable = [Handle(3)] },
                Count = 3,
                CountKind = CountKindView.UpTo,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("without paying", heading, StringComparison.Ordinal);
        Assert.DoesNotContain("mana value", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_web_slinging_cast_says_the_creature_returned_must_be_tapped_once_the_mana_is_paid()
    {
        var chooser = new ScriptedChooser([0]);

        new Decisions(chooser).Answer(
            Seated(Card(2, "Spider-Sense", "{1}{U}", "Counter target instant spell, sorcery spell, or triggered ability.")),
            new PriorityPromptMessage
            {
                Player = Me,
                Options =
                [
                    new() { Kind = ActionKindView.CastSpell, Source = Handle(2), Affordable = true, AbilityIndex = 1, Description = "Web-slinging {U}", TappedAfterMana = true },
                ],
            });

        string row = chooser.Asked[0].Choices[0];
        Assert.Contains("by an alternative cost (Web-slinging {U}; you also return a tapped creature", row, StringComparison.Ordinal);
        Assert.Contains("a creature you tap for this mana can be the one returned", row, StringComparison.Ordinal);
    }

    [Fact]
    public void A_tapped_creature_returned_for_web_slinging_is_called_a_tapped_creature()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView bears = Creature(3, "Grizzly Bears", 2, 2, Me);
        ObjectView elves = Creature(4, "Llanowar Elves", 1, 1, Me);
        bears.Tapped = true;
        elves.Tapped = true;

        new Decisions(chooser).Answer(
            Seated(bears, elves),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.ReturnToHand,
                Cards = [Handle(3), Handle(4)],
                Verdict = new() { Addable = [Handle(3), Handle(4)] },
                Count = 1,
                Kind = "tapped creature",
            });

        Assert.Contains("return 1 tapped creature(s) you control to their owner's hand.", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_harmonize_tap_says_up_to_one_and_that_it_takes_generic_mana_off_the_cost()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView bears = Creature(3, "Grizzly Bears", 2, 2, Me);

        new Decisions(chooser).Answer(
            Seated(bears),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.TapToReduce,
                Cards = [Handle(3)],
                Verdict = new() { Addable = [Handle(3)] },
                Powers = [2],
                Count = 1,
                CountKind = CountKindView.UpTo,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("up to one untapped creature", heading, StringComparison.Ordinal);
        Assert.Contains("harmonize", heading, StringComparison.Ordinal);
        Assert.Contains("generic mana of the cost", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_plot_says_the_card_becomes_plotted_and_is_cast_free_later()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView bears = Creature(3, "Grizzly Bears", 2, 2, Me);

        new Decisions(chooser).Answer(
            Seated(bears),
            new ChooseCardsPromptMessage
            {
                Player = Me,
                Source = Handle(9),
                Purpose = CardChoicePurposeView.Plot,
                Cards = [Handle(3)],
                Verdict = new() { Addable = [Handle(3)] },
                Count = 1,
                CountKind = CountKindView.UpTo,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("becomes plotted", heading, StringComparison.Ordinal);
        Assert.Contains("on a later turn without paying its mana cost", heading, StringComparison.Ordinal);
        Assert.Contains("a gain, not a loss", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_search_of_another_libraries_says_whose_and_where_the_cards_go()
    {
        var chooser = new ScriptedChooser([0]);

        new Decisions(chooser).Answer(
            Seated(Card(5, "Ornithopter", "{0}"), Card(6, "Ornithopter", "{0}")),
            new SearchLibraryPromptMessage
            {
                Player = Me,
                Owner = Them,
                Count = 2,
                CountKind = CountKindView.UpTo,
                Library = [Handle(5), Handle(6)],
                Verdict = new() { Addable = [Handle(5), Handle(6)] },
                Destination = SearchDestinationView.Exile,
            });

        string heading = chooser.Asked[0].Heading;
        Assert.Contains("Opponent's library", heading, StringComparison.Ordinal);
        Assert.Contains("to go to exile", heading, StringComparison.Ordinal);
        Assert.DoesNotContain("your hand", heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_free_cast_a_paradigm_copy_and_a_madness_cast_are_told_apart()
    {
        string Asking(string verb, int amount)
        {
            var chooser = new ScriptedChooser([0]);
            new Decisions(chooser).Answer(
                Seated(In(Card(3, "Germination Practicum", "{3}{G}{G}"), ZoneKindView.Exile, Me)),
                new MayPromptMessage { Player = Me, Source = Handle(3), Verb = verb, Amount = amount });
            return chooser.Asked[0].Heading;
        }

        Assert.Contains("without paying its mana cost", Asking("CastDuringResolution", 1), StringComparison.Ordinal);
        Assert.Contains("paying all its costs", Asking("CastDuringResolution", 0), StringComparison.Ordinal);
        Assert.Contains("a copy of it without paying", Asking("CastCopyWithoutPaying", 0), StringComparison.Ordinal);
        Assert.Contains("madness cost", Asking("CastForMadnessCost", 0), StringComparison.Ordinal);
    }

    [Fact]
    public void A_target_slot_is_told_what_it_is_for_with_the_opponent_named()
    {
        var chooser = new ScriptedChooser([0]);
        ObjectView ornithopter = Creature(3, "Ornithopter", 0, 2, Them);

        new Decisions(chooser).Answer(
            Seated(ornithopter),
            new ChooseTargetPromptMessage
            {
                Player = Me,
                Ability = Handle(9),
                Candidates = [new() { Object = Handle(3) }],
                Optional = true,
                Count = 1,
                Text = "Return up to one target artifact or creature {player} controls to its owner's hand",
                ForPlayer = Them,
            });

        Assert.Contains("controls to its owner's hand", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.Contains("Opponent controls", chooser.Asked[0].Heading, StringComparison.Ordinal);
    }

    [Fact]
    public void A_choice_of_faces_is_asked_as_faces_with_the_costs_the_engine_lists()
    {
        var chooser = new ScriptedChooser([1]);

        AnswerPayload answer = new Decisions(chooser).Answer(
            Seated(In(Card(3, "Bonecrusher Giant", "{2}{R}"), ZoneKindView.Exile, Me)),
            new ChooseModesPromptMessage
            {
                Player = Me,
                Source = Handle(3),
                Heading = "Choose which face to cast",
                Modes = ["Bonecrusher Giant {2}{R}", "Stomp {1}{R}"],
                Texts = ["Whenever this creature becomes the target of a spell, it deals 2 damage to that spell's controller.", "Stomp deals 2 damage to any target."],
                Detail = "Only the face you choose is cast, with its own cost and mana value",
                Eligible = [0, 1],
                Count = 1,
            });

        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(answer).Modes);
        Assert.Contains("Choose which face to cast", chooser.Asked[0].Heading, StringComparison.Ordinal);
        Assert.Contains("Only the face you choose", chooser.Asked[0].Note, StringComparison.Ordinal);
        Assert.Equal(["Bonecrusher Giant {2}{R}", "Stomp {1}{R}"], chooser.Asked[0].Choices);
    }
}
