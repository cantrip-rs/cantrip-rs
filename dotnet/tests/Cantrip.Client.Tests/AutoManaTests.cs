using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// Which mana ability a card with more than one is tapped for while a cost is being paid.
/// <para>
/// The rule is scarcity, and scarcity is only visible from the whole board: the same Tundra pays the {W} of
/// {2}{U}{W} beside three Islands and the {U} of it beside three Islands and four Plains. Neither case can be read
/// off the card, which is why this is worth a test rather than an eye.
/// </para>
/// </summary>
public sealed class AutoManaTests
{
    [Fact]
    public void A_named_color_replaced_by_colorless_does_not_pay_that_color()
    {
        var option = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(99),
            Produces = [ManaTypeView.Colorless],
            ColorChoices = [ColorView.Red, ColorView.Blue],
        };
        Assert.Null(AutoMana.Pick([(option, ColorView.Red), (option, ColorView.Blue)], [option], Owed(generic: 0, ColorView.Red)));
        Assert.NotNull(AutoMana.Pick([(option, ColorView.Red), (option, ColorView.Blue)], [option], Owed(generic: 1)));
    }
    [Fact]
    public void A_dual_land_pays_the_colour_the_rest_of_the_board_cannot()
    {
        // 1 Tundra and 3 Islands, casting {2}{U}{W}. Nothing else makes white, so the Tundra is the white.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);
        board.Lands(3, ColorView.Blue);

        Assert.Equal(ColorView.White, board.Pick(tundra, generic: 2, ColorView.Blue, ColorView.White));
    }

    [Fact]
    public void A_dual_land_pays_the_colour_the_rest_of_the_board_has_least_of()
    {
        // The same cast with 4 Plains as well: white is now the commoner colour, so the Tundra is the blue.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);
        board.Lands(3, ColorView.Blue);
        board.Lands(4, ColorView.White);

        Assert.Equal(ColorView.Blue, board.Pick(tundra, generic: 2, ColorView.Blue, ColorView.White));
    }

    [Fact]
    public void With_no_other_sources_the_cost_alone_decides()
    {
        // Nothing else to count, so the tie falls to the requirement the cost asks for most: {U}{U} over {W}.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);

        Assert.Equal(ColorView.Blue, board.Pick(tundra, generic: 0, ColorView.Blue, ColorView.Blue, ColorView.White));
    }

    [Fact]
    public void A_source_that_could_only_add_colorless_is_no_source_of_a_colour()
    {
        // Owing {G}{U}, with an Island and a colorless-only choice elsewhere. Only this land's painful green leaves
        // the {U} for the Island; its free blue would leave the {G} to a source that cannot make green, so the life
        // is worth paying. Read as "any colour", the colorless-only choice would cover the green and the free blue win.
        Board board = new();
        ActionOptionMessage[] clicked = board.PainLand(ColorView.Blue, ColorView.Green);
        board.Land(ColorView.Blue);
        board.ColorlessOnly();

        Assert.Equal(ColorView.Green, board.Pick(clicked, generic: 0, ColorView.Green, ColorView.Blue));
    }

    [Fact]
    public void A_colour_the_cost_does_not_ask_for_is_not_chosen_over_one_it_does()
    {
        // Five other Islands, and blue is still the only half of the Tundra that pays a {U}.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);
        board.Lands(5, ColorView.Blue);

        Assert.Equal(ColorView.Blue, board.Pick(tundra, generic: 0, ColorView.Blue));
    }

    [Fact]
    public void An_any_colour_source_pays_a_coloured_symbol_before_generic()
    {
        // Five Moxen that add any colour, casting {3}{G}{G} with one {G} already paid. Plenty of the others could
        // pay the {G}, but any of them could pay generic just as well, so the clicked one pays the {G}.
        Board board = new();
        ActionOptionMessage[] mox = board.Mox();
        for (int i = 0; i < 3; i++)
        {
            board.Mox();
        }

        Assert.Equal(ColorView.Green, board.Pick(mox, generic: 3, ColorView.Green));
    }

    [Fact]
    public void A_dual_land_pays_its_colour_before_generic()
    {
        // A {B}/{G} land beside four of the same, casting {3}{G}: the green half, not the black half on generic.
        Board board = new();
        ActionOptionMessage[] land = board.Land(ColorView.Black, ColorView.Green);
        board.Lands(4, ColorView.Black, ColorView.Green);

        Assert.Equal(ColorView.Green, board.Pick(land, generic: 3, ColorView.Green));
    }

    [Fact]
    public void Generic_alone_is_paid_in_printed_order()
    {
        // Either half pays a generic, and nothing distinguishes them, so the first one does.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);

        Assert.Equal(ColorView.White, board.Pick(tundra, generic: 3));
    }

    [Fact]
    public void Life_is_not_paid_while_a_free_half_of_the_card_would_do()
    {
        // A Talisman: {T} adds {C}, or {T} and a life adds {U} or {B}. A generic is owed and blue is plentiful.
        Board board = new();
        ActionOptionMessage[] talisman = board.Talisman();
        board.Lands(3, ColorView.Blue);

        var picked = board.Take(talisman, generic: 1, ColorView.Blue);

        Assert.Null(picked.Color);
        Assert.Equal(0, picked.Option.LifeCost);
    }

    [Fact]
    public void Life_is_paid_when_the_free_half_would_pay_nothing()
    {
        // The same Talisman with only {U}{U} left: colourless pays no part of it, so the life is paid.
        Board board = new();
        ActionOptionMessage[] talisman = board.Talisman();
        board.Lands(1, ColorView.Blue);

        var picked = board.Take(talisman, generic: 0, ColorView.Blue, ColorView.Blue);

        Assert.Equal(ColorView.Blue, picked.Color);
        Assert.Equal(1, picked.Option.LifeCost);
    }

    [Fact]
    public void A_card_that_would_pay_no_part_of_the_cost_is_left_to_the_player()
    {
        // A Mox that adds {R} or {G} while {W} is owed and no generic is: neither row helps, so neither is taken.
        Board board = new();
        ActionOptionMessage[] mox = board.Land(ColorView.Red, ColorView.Green);

        Assert.Null(AutoMana.Pick(Rows(mox), board.Offered, Owed(generic: 0, ColorView.White)));
    }

    [Fact]
    public void A_dual_land_uses_the_half_that_can_pay_a_hybrid_symbol()
    {
        // A Tundra can pay {U/R} with blue, but not with white. A hybrid-only cost must still enter auto-mana.
        Board board = new();
        ActionOptionMessage[] tundra = board.Land(ColorView.White, ColorView.Blue);

        ManaOwed owed = HybridOwed(ColorView.Blue, ColorView.Red);

        Assert.True(owed.Any);
        Assert.Equal(ColorView.Blue, board.Pick(tundra, owed));
    }

    [Fact]
    public void A_source_that_cannot_pay_a_hybrid_symbol_is_left_to_the_player()
    {
        Board board = new();
        ActionOptionMessage[] land = board.Land(ColorView.White, ColorView.Green);

        Assert.Null(AutoMana.Pick(
            Rows(land),
            board.Offered,
            HybridOwed(ColorView.Blue, ColorView.Red)));
    }

    [Fact]
    public void Overlapping_hybrid_symbols_are_matched_across_the_board()
    {
        // For {U/R}{G/W}, the red source can cover the first symbol only. The clicked dual must make green, leaving
        // red for {U/R}; making blue would leave {G/W} uncovered.
        Board board = new();
        ActionOptionMessage[] dual = board.Land(ColorView.Blue, ColorView.Green);
        board.Land(ColorView.Red);

        ManaOwed owed = ManaOwed.Of(new PayCostsPromptMessage
        {
            RemainingHybrid =
            [
                new HybridSymbolView { Colors = [ColorView.Blue, ColorView.Red] },
                new HybridSymbolView { Colors = [ColorView.Green, ColorView.White] },
            ],
        })!;

        Assert.Equal(ColorView.Green, board.Pick(dual, owed));
    }

    [Fact]
    public void Hybrid_remaining_does_not_make_X_look_ready_to_raise()
    {
        ManaOwed? owed = ManaOwed.Of(new PayCostsPromptMessage
        {
            HasX = true,
            RemainingHybrid = [new HybridSymbolView { Colors = [ColorView.Blue, ColorView.Red] }],
        });

        Assert.NotNull(owed);
        Assert.Equal(0, owed.Generic);
        Assert.True(owed.Any);
    }

    [Fact]
    public void A_card_whose_abilities_make_different_mana_is_left_to_the_player()
    {
        // Arena of Glory with {R} floating: {T} for plain {R}, or {R}, {T}, exert for {R}{R} that hastens a creature.
        // Both pay the {R} owed, and which is wanted is not a question of colour.
        Board board = new();
        ActionOptionMessage[] arena = board.Land(ColorView.Red, ColorView.Red);
        arena[1].Effect = ManaSpendEffectView.GainsHaste;
        arena[1].Produces = [ManaTypeView.Red, ManaTypeView.Red];

        Assert.Null(AutoMana.Pick(Rows(arena), board.Offered, Owed(generic: 1, ColorView.Red)));
    }

    [Fact]
    public void Raising_X_is_one_generic_owed()
    {
        // With {X} and nothing left to pay, the next mana raises X, and X is generic mana like any other.
        ManaOwed? owed = ManaOwed.Of(new PayCostsPromptMessage { HasX = true });

        Assert.NotNull(owed);
        Assert.Equal(1, owed.Generic);
    }

    [Fact]
    public void A_prompt_that_is_not_paying_for_anything_owes_nothing()
    {
        // Tapping a land with priority is not paying a cost, so there is no cost to pick the more useful colour for.
        Assert.Null(ManaOwed.Of(new PriorityPromptMessage()));
    }

    private static ManaOwed Owed(int generic, params ColorView[] colored) =>
        ManaOwed.Of(new PayCostsPromptMessage { RemainingGeneric = generic, RemainingColored = [.. colored] })!;

    private static ManaOwed HybridOwed(ColorView first, ColorView second) =>
        ManaOwed.Of(new PayCostsPromptMessage
        {
            RemainingHybrid = [new HybridSymbolView { Colors = [first, second] }],
        })!;

    /// <summary>The menu's rows: one per ability, and one per colour for an ability that adds any one colour.</summary>
    private static List<(ActionOptionMessage Option, ColorView? Color)> Rows(IEnumerable<ActionOptionMessage> options)
    {
        List<(ActionOptionMessage, ColorView?)> rows = [];

        foreach (ActionOptionMessage option in options)
        {
            if (option.ProducesOfAnyOneColor > 0)
            {
                rows.AddRange(option.ColorChoices.Select(color => (option, (ColorView?)color)));
            }
            else
            {
                rows.Add((option, null));
            }
        }

        return rows;
    }

    /// <summary>
    /// A board of untapped permanents, as the prompt describes one. Everything made here is on it, because what is
    /// on the board is the whole of what the choice is made from.
    /// </summary>
    private sealed class Board
    {
        private int _handles;

        public List<ActionOptionMessage> Offered { get; } = [];

        /// <summary>One permanent tapping for each of the given colours, as separate abilities: a dual land.</summary>
        public ActionOptionMessage[] Land(params ColorView[] colors)
        {
            var source = new ViewHandle(++_handles);
            ActionOptionMessage[] abilities =
            [
                .. colors.Select(color => new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = source,
                    Produces = [(ManaTypeView)(int)color],
                }),
            ];

            Offered.AddRange(abilities);
            return abilities;
        }

        public void Lands(int count, params ColorView[] colors)
        {
            for (int i = 0; i < count; i++)
            {
                Land(colors);
            }
        }

        /// <summary>{T}: Add the first colour. {T}, Pay 1 life: Add the second.</summary>
        public ActionOptionMessage[] PainLand(ColorView free, ColorView painful)
        {
            var source = new ViewHandle(++_handles);
            ActionOptionMessage[] abilities =
            [
                new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = source,
                    Produces = [(ManaTypeView)(int)free],
                },
                new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = source,
                    AbilityIndex = 1,
                    Produces = [(ManaTypeView)(int)painful],
                    LifeCost = 1,
                },
            ];

            Offered.AddRange(abilities);
            return abilities;
        }

        /// <summary>
        /// One mana of a type chosen from a set holding colorless alone, as Squandered Resources offers with only a
        /// Strip Mine to sacrifice (CR 106.7): a choice, with nothing in <c>ColorChoices</c>.
        /// </summary>
        public void ColorlessOnly() =>
            Offered.Add(new ActionOptionMessage
            {
                Kind = ActionKindView.ActivateAbility,
                Source = new ViewHandle(++_handles),
                ProducesOfAnyOneColor = 1,
            });

        /// <summary>{T}: Add one mana of any color.</summary>
        public ActionOptionMessage[] Mox()
        {
            ActionOptionMessage[] abilities =
            [
                new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = new ViewHandle(++_handles),
                    ProducesOfAnyOneColor = 1,
                    ColorChoices = [ColorView.White, ColorView.Blue, ColorView.Black, ColorView.Red, ColorView.Green],
                },
            ];

            Offered.AddRange(abilities);
            return abilities;
        }

        /// <summary>{T}: Add {C}. {T}, Pay 1 life: Add {U} or {B}.</summary>
        public ActionOptionMessage[] Talisman()
        {
            var source = new ViewHandle(++_handles);
            ActionOptionMessage[] abilities =
            [
                new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = source,
                    Produces = [ManaTypeView.Colorless],
                },
                new ActionOptionMessage
                {
                    Kind = ActionKindView.ActivateAbility,
                    Source = source,
                    AbilityIndex = 1,
                    ProducesOfAnyOneColor = 1,
                    ColorChoices = [ColorView.Blue, ColorView.Black],
                    LifeCost = 1,
                },
            ];

            Offered.AddRange(abilities);
            return abilities;
        }

        /// <summary>What clicking a card picks. Fails the test when it picks nothing.</summary>
        public (ActionOptionMessage Option, ColorView? Color) Take(
            IReadOnlyList<ActionOptionMessage> clicked,
            int generic,
            params ColorView[] colored)
        {
            var picked = AutoMana.Pick(Rows(clicked), Offered, Owed(generic, colored));
            Assert.NotNull(picked);
            return picked.Value;
        }

        /// <summary>The colour clicking a card pays, as the player would name it.</summary>
        public ColorView Pick(IReadOnlyList<ActionOptionMessage> clicked, int generic, params ColorView[] colored)
        {
            (ActionOptionMessage option, ColorView? color) = Take(clicked, generic, colored);
            return color ?? (ColorView)(int)option.Produces[0];
        }

        public ColorView Pick(IReadOnlyList<ActionOptionMessage> clicked, ManaOwed owed)
        {
            var picked = AutoMana.Pick(Rows(clicked), Offered, owed);
            Assert.NotNull(picked);
            return picked.Value.Color ?? (ColorView)(int)picked.Value.Option.Produces[0];
        }
    }
}
