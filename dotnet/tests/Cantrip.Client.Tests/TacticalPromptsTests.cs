using Cantrip.Client.Tactics;
using Cantrip.Protocol;
using Xunit;
using static Cantrip.Client.Tests.TacticsBoard;

namespace Cantrip.Client.Tests;

/// <summary>
/// What the tactical seat answers to questions that were once read as something else: collecting evidence, the modes of a
/// "choose one or both", the faces a card may be cast as, and the targets of an ability that exiles a card to use it.
/// </summary>
public sealed class TacticalPromptsTests
{
    private static TacticalPlay Bot() => new(TacticalModel.Prior(), seed: 1);

    private static ObjectView Dead(int handle, string name, int manaValue, int owner, bool creature = true, string text = "") => new()
    {
        Handle = Handle(handle),
        Zone = ZoneKindView.Graveyard,
        ZoneOwner = owner,
        Kind = ObjectKindView.Card,
        Owner = owner,
        Controller = owner,
        Name = name,
        ManaValue = manaValue,
        OracleText = text,
        CardTypes = [creature ? CardTypeView.Creature : CardTypeView.Sorcery],
        Power = creature ? 2 : null,
        Toughness = creature ? 2 : null,
    };

    private static ChooseModesPromptMessage Evidence() => new()
    {
        Player = Me,
        Modes = ["Cast without collecting evidence", "Collect evidence 6"],
        Eligible = [0, 1],
        Count = 1,
        AdditionalCost = true,
    };

    // --- A color that benefits the seat --------------------------------------------------------------------------------

    private static ObjectView Colored(ObjectView card, ColorView color)
    {
        card.Colors = [color];
        return card;
    }

    private static ChooseColorsPromptMessage ColorChoice(string verb) => new()
    {
        Player = Me,
        Choices = [ColorView.White, ColorView.Blue, ColorView.Black, ColorView.Red, ColorView.Green],
        Count = 1,
        Verb = verb,
    };

    [Fact]
    public void Chooses_the_color_most_of_its_own_creatures_and_spells_are_for_a_color_that_benefits_it()
    {
        GameClient client = Seated([
            Colored(Creature(3, "Hill Giant", 3, 3), ColorView.Red),
            Colored(Creature(4, "Goblin", 1, 1), ColorView.Red),
            Colored(Creature(5, "Serra Angel", 4, 4, Them), ColorView.White),
            Colored(Creature(6, "Elite Vanguard", 2, 1, Them), ColorView.White),
            Colored(Creature(7, "Knight", 2, 2, Them), ColorView.White),
            InHand(8, "Island Hopper", CardTypeView.Creature, 2, power: 1, toughness: 1, color: ColorView.Blue),
        ]);

        Assert.Equal([ColorView.Red], Assert.IsType<ColorsChosenAnswer>(Bot().Decide(client, ColorChoice("Benefit"))).Colors);
    }

    [Fact]
    public void Chooses_the_color_its_opponents_have_most_of_when_the_choice_says_nothing_of_what_it_is_for()
    {
        GameClient client = Seated([
            Colored(Creature(3, "Hill Giant", 3, 3), ColorView.Red),
            Colored(Creature(5, "Serra Angel", 4, 4, Them), ColorView.White),
            Colored(Creature(6, "Elite Vanguard", 2, 1, Them), ColorView.White),
        ]);

        Assert.Equal([ColorView.White], Assert.IsType<ColorsChosenAnswer>(Bot().Decide(client, ColorChoice(string.Empty))).Colors);
    }

    // --- Collect evidence -------------------------------------------------------------------------------------------

    [Fact]
    public void Collects_evidence_for_a_bonus_that_takes_a_card_from_an_opponents_graveyard()
    {
        GameClient client = Seated([Dead(3, "Ornithopter", 0, Them), Dead(4, "Shock", 1, Me, creature: false)]);

        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Evidence())).Modes);
    }

    [Fact]
    public void Does_not_collect_evidence_with_nothing_in_an_opponents_graveyard_for_the_bonus_to_take()
    {
        GameClient client = Seated([Dead(4, "Shock", 1, Me, creature: false)]);

        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Evidence())).Modes);
    }

    [Fact]
    public void Keeps_a_creature_in_its_own_graveyard_rather_than_collect_evidence_with_it()
    {
        GameClient client = Seated([Dead(3, "Ornithopter", 0, Them), Dead(4, "Griselbrand", 8, Me)]);

        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Evidence())).Modes);
    }

    // --- Choose one or both -----------------------------------------------------------------------------------------

    private static ChooseModesPromptMessage OneOrBoth(bool extraModesCost) => new()
    {
        Player = Me,
        Modes = ["Lightning deals 3 damage to any target.", "Destroy target creature."],
        Eligible = [0, 1],
        Count = 1,
        AtMost = 2,
        ExtraModesCost = extraModesCost,
    };

    [Fact]
    public void Takes_both_modes_of_a_one_or_both_when_neither_costs_extra_and_both_are_worth_doing()
    {
        GameClient client = Seated([Creature(3, "Giant", 4, 3, Them)]);

        Assert.Equal(2, Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, OneOrBoth(extraModesCost: false))).Modes.Count);
    }

    [Fact]
    public void Takes_one_mode_when_the_ones_past_the_first_cost_something()
    {
        GameClient client = Seated([Creature(3, "Giant", 4, 3, Them)]);

        Assert.Single(Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, OneOrBoth(extraModesCost: true))).Modes);
    }

    // --- Pawprints (CR 700.2i) -----------------------------------------------------------------------------------------

    private static ChooseModesPromptMessage Season() => new()
    {
        Player = Me,
        Modes = ["Draw a card.", "Destroy target creature.", "Each opponent loses 3 life."],
        Eligible = [0, 1, 2],
        AtMost = 5,
        MayRepeat = true,
        Pawprints = [1, 2, 3],
    };

    [Fact]
    public void Spends_every_pawprint_on_a_spread_of_modes()
    {
        GameClient client = Seated([Creature(3, "Giant", 4, 3, Them)]);
        ChooseModesPromptMessage prompt = Season();

        List<int> modes = Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, prompt)).Modes;

        Assert.Equal(5, modes.Sum(prompt.Weight));
        Assert.True(modes.Distinct().Count() > 1, string.Join(",", modes));
    }

    [Fact]
    public void A_pawprint_default_spends_them_all_dearest_first()
    {
        Assert.Equal([1, 2], Assert.IsType<ModesChosenAnswer>(Season().Default()).Modes);
        ChooseModesPromptMessage single = Season();
        single.Eligible = [0];
        Assert.Equal([0, 0, 0, 0, 0], Assert.IsType<ModesChosenAnswer>(single.Default()).Modes);
    }

    // --- A choice between discarding one card or two ------------------------------------------------------------------

    // The host sends no Texts for a card's own labels (Label::Card), so the faces branch does not answer first.
    private static ChooseModesPromptMessage Discards(string first) => new()
    {
        Player = Me,
        Modes = [first, "Discard two cards"],
        Eligible = [0, 1],
        Count = 1,
    };

    [Fact]
    public void Discards_one_card_rather_than_two_when_the_one_card_discard_is_offered()
    {
        GameClient client = Seated([Creature(3, "Giant", 4, 3, Them)]);

        ModesChosenAnswer answer = Assert.IsType<ModesChosenAnswer>(Bot().Decide(client,
            Discards("Discard an instant or sorcery card or a creature card with flying")));

        Assert.Equal([0], answer.Modes);
    }

    [Fact]
    public void Does_not_apply_the_one_card_discard_rule_to_other_modes()
    {
        GameClient client = Seated([Creature(3, "Giant", 4, 3, Them)]);

        ModesChosenAnswer answer = Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Discards("Draw a card")));

        Assert.Single(answer.Modes);
    }

    // --- Rottenmouth Viper: "each opponent loses 4 life unless that player sacrifices a nonland permanent ... or discards" -

    private static ChooseModesPromptMessage Punished() => new()
    {
        Player = Me,
        Modes = ["Sacrifice a nonland permanent", "Discard a card", "Lose 4 life"],
        Eligible = [0, 1, 2],
        Count = 1,
    };

    [Fact]
    public void Pays_the_punishment_in_life_while_life_is_comfortable()
    {
        GameClient client = Seated([InHand(1, "Island", CardTypeView.Land, 0, ""), Creature(2, "Giant", 4, 3, Me)], myLife: 20);

        Assert.Equal([2], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Punished())).Modes);
    }

    [Fact]
    public void Discards_rather_than_lose_the_last_of_its_life()
    {
        GameClient client = Seated([InHand(1, "Island", CardTypeView.Land, 0, ""), Creature(2, "Giant", 4, 3, Me)], myLife: 4);

        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Punished())).Modes);
    }

    [Fact]
    public void Sacrifices_when_it_cannot_afford_the_life_and_has_no_card_to_discard()
    {
        GameClient client = Seated([Creature(2, "Giant", 4, 3, Me)], myLife: 3);
        ChooseModesPromptMessage prompt = Punished();
        prompt.Modes = ["Sacrifice a nonland permanent", "Lose 4 life"];
        prompt.Eligible = [0, 1];

        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, prompt)).Modes);
    }

    // --- Chandra, Spark Hunter's +2: "You may sacrifice an artifact or discard a card. If you do, draw a card." ---------

    private static ChooseModesPromptMessage SacrificeOrDiscard() => new()
    {
        Player = Me,
        Modes = ["Sacrifice an artifact", "Discard a card", "Neither"],
        Eligible = [0, 1, 2],
        Count = 1,
    };

    [Fact]
    public void Declines_the_optional_sacrifice_or_discard_with_a_small_hand()
    {
        GameClient client = Seated([InHand(1, "Island", CardTypeView.Land, 0, ""), InHand(2, "Island", CardTypeView.Land, 0, "")]);

        Assert.Equal([2], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, SacrificeOrDiscard())).Modes);
    }

    [Fact]
    public void Discards_a_spare_card_for_the_draw_when_the_hand_is_full_but_never_sacrifices()
    {
        GameClient client = Seated([.. Enumerable.Range(1, 5).Select(n => InHand(n, "Island", CardTypeView.Land, 0, ""))]);

        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, SacrificeOrDiscard())).Modes);
    }

    [Fact]
    public void Takes_neither_with_a_full_hand_when_there_is_nothing_to_discard_offered()
    {
        GameClient client = Seated([.. Enumerable.Range(1, 5).Select(n => InHand(n, "Island", CardTypeView.Land, 0, ""))]);
        ChooseModesPromptMessage prompt = SacrificeOrDiscard();
        prompt.Eligible = [0, 2];

        Assert.Equal([2], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, prompt)).Modes);
    }

    // --- The faces a card may be cast as ----------------------------------------------------------------------------

    private static ChooseModesPromptMessage Faces() => new()
    {
        Player = Me,
        Heading = "Choose which face to cast",
        Modes = ["Stomp {1}{R}", "Bonecrusher Giant {2}{R}"],
        Texts = ["Stomp deals 2 damage to any target.", string.Empty],
        Eligible = [0, 1],
        Count = 1,
    };

    [Fact]
    public void Casts_the_face_that_removes_what_is_worth_removing_and_otherwise_the_creature()
    {
        GameClient prey = Seated([Creature(3, "Dragon", 6, 2, Them, "Flying")]);
        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(prey, Faces())).Modes);

        // Nothing for the damage to do but hit a player at twenty life: the body is worth more.
        GameClient nothing = Seated([]);
        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(nothing, Faces())).Modes);
    }

    // --- A card exiled to be used -----------------------------------------------------------------------------------

    [Fact]
    public void Aims_an_ability_that_exiles_a_card_of_its_own_graveyard_to_copy_it_at_a_card_worth_a_copy()
    {
        GameClient client = Seated([Dead(3, "Griselbrand", 8, Me), Land(9)]);

        TargetsChosenAnswer to = Assert.IsType<TargetsChosenAnswer>(Bot().Decide(client, new ChooseTargetPromptMessage
        {
            Player = Me,
            Text = "Exile up to one target nonland card from your graveyard of a card type none of your other targets has, and copy it",
            Candidates = [At(3)],
            Optional = true,
            Count = 1,
        }));
        Assert.Equal(At(3).Object, Assert.Single(to.Targets).Object);

        // The same exile with no use made of the card is one given up for nothing.
        TargetsChosenAnswer away = Assert.IsType<TargetsChosenAnswer>(Bot().Decide(client, new ChooseTargetPromptMessage
        {
            Player = Me,
            Text = "Exile up to one target card from your graveyard.",
            Candidates = [At(3)],
            Optional = true,
            Count = 1,
        }));
        Assert.Empty(away.Targets);
    }

    // --- A bite: one creature deals damage equal to its power to another -----------------------------------------------------

    [Fact]
    public void Bites_with_its_strongest_creature_at_the_best_creature_that_much_damage_kills()
    {
        // Its own Hill Giant (4/4) and Bear (1/1); the opponent's Dragon (6/6, too big for four damage), Bear (3/3) and Elf (1/1).
        GameClient client = Seated([
            Creature(3, "Hill Giant", 4, 4, Me), Creature(4, "Bear", 1, 1, Me),
            Creature(5, "Dragon", 6, 6, Them, "Flying"), Creature(6, "Bear", 3, 3, Them), Creature(7, "Elf", 1, 1, Them)]);
        TacticalPlay bot = Bot();

        TargetsChosenAnswer biter = Assert.IsType<TargetsChosenAnswer>(bot.Decide(client, new ChooseTargetPromptMessage
        {
            Player = Me,
            Text = "Creature you control to deal damage equal to its power",
            Candidates = [At(3), At(4)],
            Count = 1,
        }));
        Assert.Equal(At(3).Object, Assert.Single(biter.Targets).Object);

        TargetsChosenAnswer victim = Assert.IsType<TargetsChosenAnswer>(bot.Decide(client, new ChooseTargetPromptMessage
        {
            Player = Me,
            Text = "Creature or planeswalker you don't control to be dealt that damage",
            Candidates = [At(5), At(6), At(7)],
            Count = 1,
        }));
        Assert.Equal(At(6).Object, Assert.Single(victim.Targets).Object);
    }

    [Fact]
    public void Reads_the_two_slots_of_a_bite_as_helping_the_first_and_damaging_the_second()
    {
        Assert.Equal(EffectKind.Help, CardReading.Effect("Creature you control to deal damage equal to its power").Kind);
        Assert.Equal(EffectKind.Damage, CardReading.Effect("Creature or planeswalker you don't control to be dealt that damage").Kind);
    }

    // --- Which miracle to reveal a card with ------------------------------------------------------------------------

    [Fact]
    public void Reveals_a_card_with_its_cheapest_miracle_and_never_an_unpayable_one()
    {
        GameClient client = Seated([Land(9)]);
        ChooseModesPromptMessage Miracles(params string[] modes) => new()
        {
            Player = Me,
            Modes = [.. modes],
            Eligible = [.. Enumerable.Range(0, modes.Length)],
            Count = 1,
            AtMost = 1,
            Heading = "Choose which miracle to reveal it with",
        };

        // Metamorphosis Fanatic's own {1}{B} and Molecule Man's {0}.
        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Miracles("Miracle {1}{B}, its own", "Miracle {0}, from Molecule Man"))).Modes);
        // A cost of nothing to pay is no cost at all: Aminatou's for a card with no mana cost.
        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Miracles(
            "Miracle (unpayable: it has no mana cost), from Aminatou, Veil Piercer", "Miracle {2}, from Lorehold, the Historian"))).Modes);
        // The server words Aminatou's cost already reduced: {1}{W}{W} is three, dearer than Lorehold's {2}; {W} is one, cheaper;
        // what X's reduction says in brackets is not mana.
        Assert.Equal([1], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Miracles(
            "Miracle {1}{W}{W}, from Aminatou, Veil Piercer", "Miracle {2}, from Lorehold, the Historian"))).Modes);
        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Miracles(
            "Miracle {W}, from Aminatou, Veil Piercer", "Miracle {2}, from Lorehold, the Historian"))).Modes);
        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, Miracles(
            "Miracle {X}{B}{B} (X less 4), from Aminatou, Veil Piercer", "Miracle {2}{B}{B}, its own"))).Modes);
    }

    // --- Choose a creature type -------------------------------------------------------------------------------------

    private static ChooseModesPromptMessage CreatureType() => new()
    {
        Player = Me,
        Modes = ["Elf", "Goblin", "Wizard"],
        Eligible = [0, 1, 2],
        Count = 1,
        AtMost = 1,
        Heading = "Choose a creature type",
    };

    private static ObjectView Typed(ObjectView card, params string[] subtypes)
    {
        card.Subtypes = [.. subtypes];
        return card;
    }

    [Fact]
    public void Names_the_creature_type_it_has_most_of_on_the_battlefield_and_in_hand()
    {
        // Roaming Throne's choice: two Wizards (one in hand) beat one Goblin, and an opponent's Elves count for nothing.
        GameClient client = Seated([
            Typed(Creature(3, "Goblin Guide", 2, 2), "Goblin"),
            Typed(Creature(4, "Snapcaster Mage", 2, 1), "Human", "Wizard"),
            Typed(InHand(5, "Tersa Lightshatter", CardTypeView.Creature, 3, power: 3, toughness: 3), "Orc", "Wizard"),
            Typed(Creature(6, "Llanowar Elves", 1, 1, controller: Them), "Elf"),
            Typed(Creature(7, "Elvish Mystic", 1, 1, controller: Them), "Elf"),
            Typed(Creature(8, "Fyndhorn Elves", 1, 1, controller: Them), "Elf"),
        ]);

        Assert.Equal([2], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, CreatureType())).Modes);
    }

    [Fact]
    public void Names_the_first_creature_type_offered_with_none_of_any()
    {
        GameClient client = Seated([Typed(Creature(3, "Ornithopter", 0, 2), "Thopter")]);

        Assert.Equal([0], Assert.IsType<ModesChosenAnswer>(Bot().Decide(client, CreatureType())).Modes);
    }
}
