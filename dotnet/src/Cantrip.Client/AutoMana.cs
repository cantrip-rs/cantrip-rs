using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>
/// What a cost still owes, whichever prompt is asking for it: paying for a spell, or paying to stop one.
/// <para>
/// The two prompts describe what is left the same way — generic, and one entry per coloured symbol — so the thing
/// that reads them says so once rather than twice.
/// </para>
/// </summary>
public sealed class ManaOwed
{
    private ManaOwed(
        int generic,
        IReadOnlyList<ColorView> colored,
        IReadOnlyList<HybridSymbolView> hybrid,
        int colorless,
        uint snow = 0)
    {
        Generic = generic;
        Colored = colored;
        Hybrid = hybrid;
        Colorless = colorless;
        Snow = snow;
    }

    /// <summary>Generic mana still owed. Any mana at all pays it.</summary>
    public int Generic { get; }

    /// <summary>Coloured symbols still owed, one entry per symbol: <c>{U}{U}</c> is two entries.</summary>
    public IReadOnlyList<ColorView> Colored { get; }

    /// <summary>Hybrid and Phyrexian symbols still owed, each payable with one of its colours (also life, for Phyrexian).</summary>
    public IReadOnlyList<HybridSymbolView> Hybrid { get; }

    /// <summary>
    /// Colorless symbols {C} still owed (CR 107.4c). Only colorless mana pays one, so it is a requirement of its own
    /// rather than part of <see cref="Generic"/>. Always zero for an "unless you pay" cost, whose prompt does not carry it.
    /// </summary>
    public int Colorless { get; }

    /// <summary>Snow symbols require mana produced by a snow source, of any type.</summary>
    public uint Snow { get; }

    /// <summary>Whether anything is owed at all.</summary>
    public bool Any => Generic > 0 || Colored.Count > 0 || Hybrid.Count > 0 || Colorless > 0 || Snow > 0;

    /// <summary>
    /// What the prompt on the board still owes, or null when the prompt is not one that mana is being paid to.
    /// <para>
    /// A spell with {X} that owes nothing is owed one generic: the next mana raises X, and X is generic mana, so a
    /// land tapped for it is chosen on exactly the grounds a land tapped for the rest of the cost would be.
    /// </para>
    /// </summary>
    public static ManaOwed? Of(PromptPayload? prompt) => prompt switch
    {
        // The announcement of X (CR 601.2b) is not a payment: nothing is paid until the targets are chosen.
        PayCostsPromptMessage { AnnouncesXOrKicks: false } pay => new ManaOwed(
            pay.RemainingGeneric + (pay.HasX && !pay.ManaAbilities
                && pay.RemainingGeneric == 0
                && pay.RemainingColored.Count == 0
                && pay.RemainingHybrid.Count == 0
                && pay.RemainingColorless == 0 && pay.RemainingSnow == 0 ? 1 : 0),
            pay.RemainingColored,
            pay.RemainingHybrid,
            pay.RemainingColorless, pay.RemainingSnow),
        UnlessPaysPromptMessage unless => new ManaOwed(
            unless.RemainingGeneric,
            unless.RemainingColored,
            unless.RemainingHybrid,
            colorless: 0, snow: unless.RemainingSnow),
        _ => null,
    };

    /// <summary>How many remaining symbols a colour could pay, including either half of a hybrid symbol.</summary>
    public int CountOf(ColorView color) =>
        Colored.Count(owed => owed == color) + Hybrid.Count(owed => owed.Accepts(color));
}

/// <summary>
/// Which mana ability to use when a card with more than one is clicked while a cost is being paid, so the player
/// does not read a menu for a choice that only has one sensible answer.
/// <para>
/// The rule is scarcity. Every mana ability on the board is on the prompt, so the client can count, for each thing the
/// cost still asks for, how many <em>other</em> permanents could supply it. The clicked card is then spent on
/// whichever requirement the rest of the board is least able to cover: a Tundra tapped for {W} when the other lands
/// are Islands, and for {U} when there are Plains to spare.
/// </para>
/// <para>
/// Scarcity is counted as slack — other sources minus symbols owed — so a requirement asked for twice counts as
/// twice as tight. Generic is not ranked by slack against a coloured symbol, though: a card that can pay a symbol
/// pays it, because the source that would have paid it can pay the generic instead. Comparing the two by slack
/// spent the second of five any-colour Moxen on the generic of {3}{G}{G}, with the {G} still owed. Past a symbol
/// the rest of the board is short of, an ability that costs nothing comes before one that costs life or damage,
/// then coloured before generic, the tightest slack, and printed order. Nothing here decides which card to tap.
/// That is the player's click; this only decides what the card they tapped is for.
/// </para>
/// <para>
/// A source is counted for a requirement it could produce the mana for, whatever its mana may then be spent on: a
/// restricted source that cannot in fact pay for this spell is counted as though it could, which makes the colour it
/// produces look one commoner than it is. The server refuses nothing here — every answer this picks is one the
/// menu offered — so the cost of being wrong is a land tapped for the less useful of two colours, and the player
/// can take it back with Undo.
/// </para>
/// </summary>
public static class AutoMana
{
    /// <summary>
    /// Which of a card's offered rows to take, or null when nothing it offers would pay any part of what is owed and
    /// the player had better be asked. Rows are the menu's rows: one per ability, and one per colour for an ability
    /// that adds mana of any one colour.
    /// </summary>
    /// <param name="rows">What the clicked card offers.</param>
    /// <param name="offered">Everything the prompt offers, which is how the rest of the board is counted.</param>
    /// <param name="owed">What the cost still asks for.</param>
    public static (ActionOptionMessage Option, ColorView? Color)? Pick(
        IReadOnlyList<(ActionOptionMessage Option, ColorView? Color)> rows,
        IReadOnlyList<ActionOptionMessage> offered,
        ManaOwed owed)
    {
        ArgumentNullException.ThrowIfNull(rows);
        ArgumentNullException.ThrowIfNull(offered);
        ArgumentNullException.ThrowIfNull(owed);

        // The clicked card is not one of the other sources. Its own abilities are rivals for the same untap, not
        // spare mana: a Tundra is one mana whichever colour it is tapped for.
        HashSet<int> clicked = [.. rows.Select(row => row.Option.Source.Value)];

        // Rows whose mana is different mana — Arena of Glory's plain {R} beside its {R}{R} that gives a creature haste —
        // are a choice about more than colour, and the player makes it.
        if (rows.Where(row => IsManaAbility(row.Option)).Select(row => (row.Option.Filter, row.Option.Effect, string.Join(",", row.Option.SpendSubtypes), row.Option.SpendManaValue)).Distinct().Count() > 1)
        {
            return null;
        }

        (ActionOptionMessage Option, ColorView? Color)? best = null;
        (int Short, int Costly, int Generic, int Slack, int Order) bestRank = default;

        foreach ((ActionOptionMessage option, ColorView? color) in rows)
        {
            if (!IsManaAbility(option))
            {
                continue;
            }

            if (Rank(option, color, offered, owed, clicked) is not { } rank)
            {
                continue;
            }

            if (best is null || rank.CompareTo(bestRank) < 0)
            {
                best = (option, color);
                bestRank = rank;
            }
        }

        return best;
    }

    /// <summary>
    /// Whether this option is an ability that adds mana, as opposed to one that uses the stack. The other side of
    /// the renderer's own "an activation that cannot be taken back", said here so that choosing a mana ability needs
    /// nothing from a window and can be checked on its own.
    /// </summary>
    public static bool IsManaAbility(ActionOptionMessage option)
    {
        ArgumentNullException.ThrowIfNull(option);
        return option.Kind == ActionKindView.ActivateAbility
            && (option.Produces.Count > 0 || option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0);
    }

    /// <summary>
    /// How good a row is, lower being better, or null when it would pay no part of what is owed — a Talisman's
    /// colourless half while only coloured symbols are left.
    /// <para>
    /// A row that can pay a coloured symbol pays it rather than generic, always: whatever else would have paid that
    /// symbol can pay the generic instead, so nothing is lost, and the reverse is not true. Between rows, in order:
    /// a symbol the rest of the board is short of, then the free row over the one costing life or damage, then a
    /// coloured symbol over generic, then the tightest slack, then printed order.
    /// </para>
    /// </summary>
    private static (int Short, int Costly, int Generic, int Slack, int Order)? Rank(
        ActionOptionMessage option,
        ColorView? color,
        IReadOnlyList<ActionOptionMessage> offered,
        ManaOwed owed,
        HashSet<int> clicked)
    {
        // Mana a restriction keeps from paying this cost (CR 106.6) pays none of it: the player is asked instead.
        if (option.PaysNothing)
        {
            return null;
        }

        (int Missing, int Slack, int Order)? fit = null;

        foreach (ColorView symbol in Adds(option, color))
        {
            int wanted = owed.CountOf(symbol);
            if (wanted == 0)
            {
                continue;
            }

            foreach (int requirement in RequirementsPaidBy(owed, symbol))
            {
                (int Missing, int Slack, int Order) here = (
                    MissingAfterPaying(owed, offered, clicked, requirement),
                    Sources(offered, clicked, symbol) - wanted,
                    (int)symbol);

                if (fit is null || here.CompareTo(fit.Value) < 0)
                {
                    fit = here;
                }
            }
        }

        int costly = option.LifeCost + option.DamageToController > 0 ? 1 : 0;
        if (fit is { } colored)
        {
            return (colored.Missing, costly, 0, colored.Slack, colored.Order);
        }

        // A {C} still owed (CR 107.4c) is a symbol like a colour's, which only colorless mana pays: ranked beside the
        // coloured ones rather than as generic, after every colour in printed order.
        if (owed.Colorless > 0 && option.Produces.Contains(ManaTypeView.Colorless))
        {
            int others = offered
                .Where(other => IsManaAbility(other)
                    && !clicked.Contains(other.Source.Value)
                    && other.Produces.Contains(ManaTypeView.Colorless))
                .Select(other => other.Source.Value)
                .Distinct()
                .Count();

            return (0, costly, 0, others - owed.Colorless, (int)ManaTypeView.Colorless);
        }

        if (owed.Snow > 0 && option.Snow)
        {
            int others = offered.Where(other => IsManaAbility(other) && other.Snow
                && !other.PaysNothing && !clicked.Contains(other.Source.Value))
                .Select(other => other.Source.Value).Distinct().Count();
            return (0, costly, 0, others - (int)Math.Min(owed.Snow, int.MaxValue), 6);
        }

        // Every source in play pays generic, so it is only ever the last resort of a row that fits nothing better.
        return owed.Generic > 0
            ? (0, costly, 1, Sources(offered, clicked, color: null) - owed.Generic, 0)
            : null;
    }

    /// <summary>
    /// The requirements a colour can pay. Fixed-colour requirements come first, followed by hybrid symbols, matching
    /// the order used by <see cref="MissingAfterPaying"/>.
    /// </summary>
    private static IEnumerable<int> RequirementsPaidBy(ManaOwed owed, ColorView color)
    {
        for (int i = 0; i < owed.Colored.Count; i++)
        {
            if (owed.Colored[i] == color)
            {
                yield return i;
            }
        }

        for (int i = 0; i < owed.Hybrid.Count; i++)
        {
            if (owed.Hybrid[i].Accepts(color))
            {
                yield return owed.Colored.Count + i;
            }
        }
    }

    /// <summary>
    /// How many coloured or hybrid requirements the other permanents cannot cover after the clicked mana pays one.
    /// Matching matters for overlapping hybrids: beside a red source, green rather than blue must pay one symbol of
    /// <c>{U/R}{U/G}</c>, leaving the red source able to pay the other.
    /// </summary>
    private static int MissingAfterPaying(
        ManaOwed owed,
        IReadOnlyList<ActionOptionMessage> offered,
        HashSet<int> clicked,
        int paidRequirement)
    {
        List<(ColorView First, ColorView? Second)> requirements =
        [
            .. owed.Colored.Select(color => (color, (ColorView?)null)),
            .. owed.Hybrid.Select(symbol =>
                (symbol.Colors[0], symbol.Colors.Count > 1 ? (ColorView?)symbol.Colors[1] : null)),
        ];
        requirements.RemoveAt(paidRequirement);

        List<ActionOptionMessage[]> sources =
        [
            .. offered
                .Where(option => IsManaAbility(option) && !clicked.Contains(option.Source.Value))
                .GroupBy(option => option.Source.Value)
                .Select(group => group.ToArray()),
        ];

        int[] sourceForRequirement = Enumerable.Repeat(-1, requirements.Count).ToArray();
        int covered = 0;
        for (int source = 0; source < sources.Count; source++)
        {
            if (Match(source, new bool[requirements.Count]))
            {
                covered++;
            }
        }

        return requirements.Count - covered;

        bool Match(int source, bool[] seen)
        {
            for (int requirement = 0; requirement < requirements.Count; requirement++)
            {
                if (seen[requirement] || !CanPay(sources[source], requirements[requirement]))
                {
                    continue;
                }

                seen[requirement] = true;
                if (sourceForRequirement[requirement] < 0
                    || Match(sourceForRequirement[requirement], seen))
                {
                    sourceForRequirement[requirement] = source;
                    return true;
                }
            }

            return false;
        }
    }

    private static bool CanPay(
        IReadOnlyList<ActionOptionMessage> source,
        (ColorView First, ColorView? Second) requirement) =>
        source.Any(option => CouldAdd(option, requirement.First)
            || requirement.Second is { } second && CouldAdd(option, second));

    /// <summary>
    /// The colours a row adds. An ability of any one colour adds the colour of its row, and whatever else it adds
    /// besides — a Birds of Paradise is one row per colour, a hypothetical <c>{C}</c> plus any one colour is five.
    /// </summary>
    private static IEnumerable<ColorView> Adds(ActionOptionMessage option, ColorView? color)
    {
        foreach (ManaTypeView type in option.Produces)
        {
            if (type != ManaTypeView.Colorless)
            {
                yield return (ColorView)(int)type;
            }
        }

        if (color is { } chosen && (option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0))
        {
            yield return chosen;
        }
    }

    /// <summary>
    /// How many permanents other than the clicked one could pay a requirement: a named colour, or, for null, generic,
    /// which any mana pays. Counted per permanent rather than per ability, because a permanent with two mana
    /// abilities is still only untapped once.
    /// </summary>
    private static int Sources(IReadOnlyList<ActionOptionMessage> offered, HashSet<int> clicked, ColorView? color)
    {
        HashSet<int> sources = [];

        foreach (ActionOptionMessage option in offered)
        {
            if (!IsManaAbility(option) || clicked.Contains(option.Source.Value))
            {
                continue;
            }

            if (color is not { } wanted || CouldAdd(option, wanted))
            {
                sources.Add(option.Source.Value);
            }
        }

        return sources.Count;
    }

    /// <summary>
    /// Whether an ability could add mana of a colour, counting the colours it offers a choice between. The server
    /// always lists those colours, so a choice with none listed is a choice of colorless alone (CR 106.7, a Squandered
    /// Resources candidate that could only add {C}), never "any colour".
    /// </summary>
    private static bool CouldAdd(ActionOptionMessage option, ColorView color) =>
        option.Produces.Contains((ManaTypeView)(int)color)
        || ((option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0)
            && option.ColorChoices.Contains(color));
}
