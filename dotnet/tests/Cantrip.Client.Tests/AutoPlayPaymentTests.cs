using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// Which land a bot taps while paying, when more than one would do.
/// <para>
/// <see cref="AutoMana"/> answers the player's version of this question — which row of the card they clicked — and
/// is tested next door. This is the bot's version, which nobody clicks: given everything the prompt offers, which
/// ability to activate next. The two are separate because the player's has a card already chosen and the bot's does
/// not.
/// </para>
/// <para>
/// The stake is not elegance. A cost the server called affordable and the bot then cannot pay is given up on
/// (CR 733.1), which puts the game back exactly where it was with the same offer on the menu — harmless for a seat
/// that draws at random and re-draws, and an unbounded loop for one that decides.
/// </para>
/// </summary>
public sealed class AutoPlayPaymentTests
{
    [Fact]
    public void Closes_mana_window_when_the_pool_covers_the_cost()
    {
        var prompt = new PayCostsPromptMessage
        {
            ManaAbilities = true,
            HasX = true,
            Total = "{1}",
            RemainingGeneric = 0,
            Options =
            [
                new ActionOptionMessage { Kind = ActionKindView.ActivateAbility, Source = new ViewHandle(2), Produces = [ManaTypeView.Green] },
                new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting },
                new ActionOptionMessage { Kind = ActionKindView.CancelCasting },
            ],
        };
        Assert.Equal(ActionKindView.ConfirmCasting, AutoPlay.Pay(prompt).Kind);
        Assert.False(ManaOwed.Of(prompt)!.Any, "the announced X is already included in the cost");
    }

    /// <summary>
    /// The announcement of X (CR 601.2b), which comes before the targets and offers nothing to pay with: the bot steps
    /// to the most the server says it could pay for, then announces it, and never answers with mana.
    /// </summary>
    [Theory]
    [InlineData(0, 2, ActionKindView.IncreaseX)]
    [InlineData(2, 2, ActionKindView.ConfirmCasting)]
    [InlineData(3, 2, ActionKindView.DecreaseX)]
    [InlineData(0, -1, ActionKindView.ConfirmCasting)]
    public void Announces_X_up_to_what_it_could_pay_for(int x, int affordable, ActionKindView expected)
    {
        var spell = new ViewHandle(1);
        var prompt = new PayCostsPromptMessage
        {
            Spell = spell,
            Total = "{X}{B}",
            HasX = true,
            X = x,
            AnnouncesXOrKicks = true,
            MostAffordable = affordable,
            RemainingGeneric = x,
            Options =
            [
                new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = spell },
                new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = spell },
                new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = spell },
                new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = spell },
            ],
        };

        Assert.Equal(expected, AutoPlay.Pay(prompt).Kind);
    }

    /// <summary>
    /// An X paid in life (CR 119.4): the server's advice is the most that leaves one life, and the bot aims at half of
    /// it rather than spending itself down to one on a single spell.
    /// </summary>
    [Theory]
    [InlineData(19, 9)]
    [InlineData(1, 0)]
    [InlineData(-1, 0)]
    public void Aims_an_X_paid_in_life_at_half_of_what_it_could_survive(int affordable, int expected)
    {
        var prompt = new PayCostsPromptMessage { AnnouncesXOrKicks = true, XPaidInLife = true, HasX = true, MostAffordable = affordable };

        Assert.Equal(expected, AutoPlay.AnnouncedValue(prompt));
    }

    [Fact]
    public void Spends_a_one_colour_land_before_the_dual_that_is_the_only_source_of_the_other_colour()
    {
        // Elegant Parlor (R or W) and two Mountains, paying {1}{R}{W}. Tapping the Parlor for red leaves {W} owed
        // with nothing but Mountains — the exact shape that looped a real game.
        var parlor = new ViewHandle(1);

        List<ActionOptionMessage> options =
        [
            Taps(parlor, ColorView.Red),
            Taps(parlor, ColorView.White),
            Taps(new ViewHandle(2), ColorView.Red),
            Taps(new ViewHandle(3), ColorView.Red),
        ];

        TakeActionAnswer? first = AutoPlay.Produce(options, [ColorView.Red, ColorView.White], owedGeneric: 1);

        Assert.NotNull(first);
        Assert.NotEqual(parlor, first.Source);
    }

    [Fact]
    public void Taps_the_dual_once_it_is_the_only_thing_left_that_pays_what_is_owed()
    {
        var parlor = new ViewHandle(1);

        List<ActionOptionMessage> options =
        [
            Taps(parlor, ColorView.Red),
            Taps(parlor, ColorView.White),
            Taps(new ViewHandle(2), ColorView.Red),
        ];

        // The red is covered and only the white is left: now the dual is the whole answer, and for white.
        TakeActionAnswer? answer = AutoPlay.Produce(options, [ColorView.White], owedGeneric: 0);

        Assert.NotNull(answer);
        Assert.Equal(parlor, answer.Source);
    }

    [Fact]
    public void Pays_a_cost_out_in_full_without_stranding_a_colour()
    {
        // The whole payment, one tap at a time, the way the prompt actually drives it: every land is spent and
        // nothing is left owed. This is the property that matters; which land went first is only how it is reached.
        // Distinct ability indexes, as a real dual's two rows carry, so the answer says which colour was tapped for.
        List<ActionOptionMessage> options =
        [
            Taps(new ViewHandle(1), ColorView.Red, ability: 0),
            Taps(new ViewHandle(1), ColorView.White, ability: 1),
            Taps(new ViewHandle(2), ColorView.Red),
            Taps(new ViewHandle(3), ColorView.Red),
        ];

        List<ColorView> owedColors = [ColorView.Red, ColorView.White];
        int owedGeneric = 1;
        HashSet<int> tapped = [];

        for (int tap = 0; tap < 3; tap++)
        {
            List<ActionOptionMessage> untapped = [.. options.Where(option => !tapped.Contains(option.Source.Value))];
            TakeActionAnswer? answer = AutoPlay.Produce(untapped, owedColors, owedGeneric);

            Assert.True(answer is not null, $"gave up with {string.Join(",", owedColors)} and {owedGeneric} generic still owed");

            tapped.Add(answer.Source.Value);

            // What that mana pays: its colour if that colour is owed, otherwise the generic.
            ActionOptionMessage taken = untapped.First(option =>
                option.Source == answer.Source && option.AbilityIndex == answer.AbilityIndex);

            var made = (ColorView)(int)taken.Produces[0];

            if (owedColors.Contains(made))
            {
                owedColors.Remove(made);
            }
            else
            {
                owedGeneric--;
            }
        }

        Assert.Empty(owedColors);
        Assert.Equal(0, owedGeneric);
    }

    /// <summary>
    /// The pool covers a convoke spell and a creature could still be tapped for it (CR 601.2h): the bot pays with the
    /// pool, which empties at the end of the step anyway, rather than tapping a creature it could attack or block with.
    /// </summary>
    [Fact]
    public void Confirms_a_pool_that_covers_the_cost_rather_than_convoking()
    {
        var spell = new ViewHandle(1);
        var bear = new ViewHandle(2);
        var prompt = new PayCostsPromptMessage
        {
            Spell = spell,
            Total = "{3}{W}",
            Options =
            [
                new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = bear, Description = "Tap for {1} or {G}" },
                new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = spell },
                new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = spell },
            ],
        };

        Assert.Equal(ActionKindView.ConfirmCasting, AutoPlay.Pay(prompt).Kind);
    }

    /// <summary>
    /// A {2} waterbend ability under a +{2} activation tax, with five manaless artifacts and four Islands: the artifacts
    /// can pay only the waterbend's {2}, however many there are, and no mana ability is offered once one has been
    /// tapped (CR 601.2g-h). So the bot makes the tax's {2} in mana first; counting all five artifacts against the {4}
    /// owed, it would tap two, still owe {2}, and have nothing left to do but give up.
    /// </summary>
    [Fact]
    public void Makes_the_mana_a_waterbend_cap_leaves_owed_before_tapping_for_it()
    {
        var ability = new ViewHandle(1);
        List<ActionOptionMessage> options = [];

        for (int i = 0; i < 4; i++)
        {
            options.Add(Taps(new ViewHandle(10 + i), ColorView.Blue));
        }

        for (int i = 0; i < 5; i++)
        {
            options.Add(new ActionOptionMessage
            {
                Kind = ActionKindView.TapToPay, Source = new ViewHandle(20 + i), PaysUpTo = 2, Alternative = 0,
            });
        }

        options.Add(new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = ability });

        var prompt = new PayCostsPromptMessage
        {
            Spell = ability,
            Total = "{4}",
            RemainingGeneric = 4,
            Options = options,
        };

        TakeActionAnswer answer = AutoPlay.Pay(prompt);

        Assert.Equal(ActionKindView.ActivateAbility, answer.Kind);
        Assert.InRange(answer.Source.Value, 10, 13);
    }

    /// <summary>
    /// "You can't spend mana to cast this spell" (CR 601.2h, 106.4), as Hogaak's: a Forest and a Llanowar Elves still
    /// offer their mana abilities (CR 601.2g), but the mana would pay nothing, and an Elves tapped for it can no longer
    /// convoke. The bot makes no mana: it exiles for delve, then taps for convoke, and only then gives up.
    /// </summary>
    [Fact]
    public void A_spell_no_mana_may_pay_for_is_paid_by_exiles_and_taps_and_never_by_making_mana()
    {
        var spell = new ViewHandle(1);
        var forest = new ViewHandle(10);
        var elves = new ViewHandle(11);
        var buried = new ViewHandle(20);
        List<ActionOptionMessage> Offered(bool exile, bool tap) =>
        [
            Taps(forest, ColorView.Green),
            Taps(elves, ColorView.Green),
            .. tap ? [new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = elves, Description = "Tap Llanowar Elves to pay {1} or {G}" }] : Array.Empty<ActionOptionMessage>(),
            .. exile ? [new ActionOptionMessage { Kind = ActionKindView.ExileToPay, Source = buried }] : Array.Empty<ActionOptionMessage>(),
            new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = spell },
        ];

        PayCostsPromptMessage Prompt(bool exile, bool tap) => new()
        {
            Spell = spell,
            Total = "{5}{B/G}{B/G}",
            RemainingGeneric = 5,
            RemainingHybrid =
            [
                new HybridSymbolView { Colors = [ColorView.Black, ColorView.Green] },
                new HybridSymbolView { Colors = [ColorView.Black, ColorView.Green] },
            ],
            SpendsNoMana = true,
            Options = Offered(exile, tap),
        };

        TakeActionAnswer first = AutoPlay.Pay(Prompt(exile: true, tap: true));
        Assert.Equal(ActionKindView.ExileToPay, first.Kind);
        Assert.Equal(buried, first.Source);

        TakeActionAnswer second = AutoPlay.Pay(Prompt(exile: false, tap: true));
        Assert.Equal(ActionKindView.TapToPay, second.Kind);
        Assert.Equal(elves, second.Source);

        Assert.Equal(ActionKindView.CancelCasting, AutoPlay.Pay(Prompt(exile: false, tap: false)).Kind);
    }

    /// <summary>
    /// Mana a restriction keeps from paying this cost (CR 106.6) - Sunken Citadel's {U}{U} for abilities of land sources,
    /// offered while a spell is paid for - is never made: it would only wait in the pool. The Citadel's own plain {U} is.
    /// </summary>
    [Fact]
    public void Leaves_mana_that_would_pay_nothing_here_unmade()
    {
        var citadel = new ViewHandle(1);
        ActionOptionMessage restricted = Taps(citadel, ColorView.Blue, ability: 1);
        restricted.Produces = [ManaTypeView.Blue, ManaTypeView.Blue];
        restricted.Filter = ManaSpendFilterView.LandAbilities;
        restricted.PaysNothing = true;

        TakeActionAnswer? answer = AutoPlay.Produce([restricted, Taps(citadel, ColorView.Blue)], [ColorView.Blue], owedGeneric: 1);

        Assert.NotNull(answer);
        Assert.Equal(0, answer.AbilityIndex);
        Assert.Null(AutoPlay.Produce([restricted], [ColorView.Blue], owedGeneric: 0));
    }

    /// <summary>
    /// A mana ability whose cost taps more than one permanent of the player's choosing (Supportive Parents: "Tap two untapped
    /// creatures you control: Add one mana of any color.") is offered with those permanents as its option's targets, since
    /// a mana ability has none of its own (CR 605.1a): the bot names as many different ones as the cost takes. An answer
    /// short of them is refused, and the same payment offered again for ever.
    /// </summary>
    [Fact]
    public void Names_every_permanent_a_mana_ability_taps_for_its_cost()
    {
        var parents = new ViewHandle(1);
        var tapsTwo = new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = parents,
            ProducesOfAnyOneColor = 1,
            ColorChoices = [ColorView.White, ColorView.Blue, ColorView.Black, ColorView.Red, ColorView.Green],
            TargetsRequired = 2,
            TargetCandidates = [.. new[] { 1, 2, 3 }.Select(handle => new TargetMessage { Object = new ViewHandle(handle) })],
        };

        TakeActionAnswer? answer = AutoPlay.Produce([tapsTwo], [ColorView.Green], owedGeneric: 0);

        Assert.NotNull(answer);
        Assert.Equal(ColorView.Green, answer.Color);
        Assert.Equal(2, answer.Targets.Select(target => target.Object).Distinct().Count());
        Assert.All(answer.Targets, target => Assert.Contains(tapsTwo.TargetCandidates, candidate => candidate.Object == target.Object));
    }

    /// <summary>
    /// Mana a rule tells apart by its source (CR 106.3) - Barracks of the Thousand's {W} beside a Plains' - is spent where the
    /// Barracks' text rewards it, on an artifact or creature spell, and kept otherwise: the plain {W} pays an enchantment.
    /// </summary>
    [Theory]
    [InlineData(CardTypeView.Creature, true)]
    [InlineData(CardTypeView.Artifact, true)]
    [InlineData(CardTypeView.Enchantment, false)]
    public void Spends_mana_told_apart_by_its_source_only_where_its_text_rewards_it(CardTypeView spellType, bool fromBarracks)
    {
        var barracks = new ObjectView
        {
            Handle = TacticsBoard.Handle(1), Name = "Barracks of the Thousand", Zone = ZoneKindView.Battlefield,
            Owner = TacticsBoard.Me, Controller = TacticsBoard.Me, CardTypes = [CardTypeView.Artifact, CardTypeView.Land],
            OracleText = "{T}: Add {W}.\nWhenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand, create a white Gnome Soldier artifact creature token.",
        };
        var spell = new ObjectView
        {
            Handle = TacticsBoard.Handle(2), Name = "Spell", Zone = ZoneKindView.Stack,
            Owner = TacticsBoard.Me, Controller = TacticsBoard.Me, CardTypes = [spellType],
        };
        GameClient client = TacticsBoard.Seated([barracks, spell]);
        var prompt = new PayCostsPromptMessage
        {
            Spell = spell.Handle,
            Total = "{W}",
            RemainingColored = [ColorView.White],
            Options =
            [
                new ActionOptionMessage { Kind = ActionKindView.AssignMana, Produces = [ManaTypeView.White] },
                new ActionOptionMessage { Kind = ActionKindView.AssignMana, Produces = [ManaTypeView.White], Source = barracks.Handle },
                new ActionOptionMessage { Kind = ActionKindView.CancelCasting },
            ],
        };

        TakeActionAnswer answer = AutoPlay.Pay(prompt, client);

        Assert.Equal(ActionKindView.AssignMana, answer.Kind);
        Assert.Equal(fromBarracks ? barracks.Handle : ViewHandle.None, answer.Source);
    }

    private static ActionOptionMessage Taps(ViewHandle source, ColorView color, int ability = 0) => new()
    {
        Kind = ActionKindView.ActivateAbility,
        Source = source,
        AbilityIndex = ability,
        Produces = [(ManaTypeView)(int)color],
    };
}
