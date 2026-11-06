using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using ManaRow = (Cantrip.Protocol.ActionOptionMessage Option, Cantrip.Protocol.ColorView? Color, System.Collections.Generic.List<Cantrip.Protocol.ColorView> Combination, Cantrip.Protocol.ViewHandle? Sacrifice);

namespace Cantrip.Client.Web;

/// <summary>
/// Mana planned for an {X} while X is announced (CR 601.2b). No mana ability may be activated then - the casting player's
/// chance to is after the targets, as the cost is paid (CR 601.2g) - so a land clicked during the announcement is not
/// tapped: it is planned for the spell, X rises to what the mana planned and the pool reach, and once the payment is asked
/// the page taps what was planned, one land an answer, before the player is asked for anything still owed.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>The mana abilities the priority question offered as a spell or ability was chosen: what a planned source adds.</summary>
    private List<ActionOptionMessage> _castMana = [];

    /// <summary>The sources planned for X, in the order clicked: tapped in that order as the cost is paid.</summary>
    private readonly List<ViewHandle> _planned = [];

    /// <summary>The spell or ability the planned sources are for, as the announcement and the payment name it.</summary>
    private ViewHandle _plannedFor;

    /// <summary>
    /// Whether the announcement is of an {X} paid in mana, so that a mana source clicked is planned for it: not a kicker,
    /// an X paid in life or loyalty, or a number of permanents sacrificed, and not a cost no mana may be spent on.
    /// </summary>
    private bool PlansMana(PayCostsPromptMessage prompt) =>
        prompt.AnnouncesXOrKicks
        && prompt.Kicker.Length == 0
        && !prompt.XPaidInLife
        && !prompt.XRemovesLoyalty
        && prompt.SacrificeReduces == 0
        && !prompt.SpendsNoMana
        && Announcement.XSymbols(prompt.Total) > 0
        && _castMana.Count > 0;

    /// <summary>The most mana one of a source's mana abilities adds, as the priority question offered them; zero for none.</summary>
    private int ManaFrom(ViewHandle source) =>
        _castMana
            .Where(option => option.Source == source)
            .Select(option => option.Produces.Count + option.ProducesOfAnyOneColor + option.ProducesOfAnyColorEach)
            .DefaultIfEmpty(0)
            .Max();

    /// <summary>The mana there is for X: the player's pool, mana any spell may be paid with, and what the planned sources add.</summary>
    private int ManaForX(GameClient client) => PoolMana(client) + _planned.Sum(ManaFrom);

    private static int PoolMana(GameClient client) =>
        client.View?.Players.FirstOrDefault(player => player.Seat == client.ActingSeat)?.Mana.Sum() ?? 0;

    /// <summary>
    /// Plans a source for X, or takes it back off the plan, and sets X to what the mana now reaches: the stepper changes it
    /// from there.
    /// </summary>
    private void TogglePlanned(GameClient client, PayCostsPromptMessage prompt, ViewHandle source)
    {
        if (!_planned.Remove(source))
        {
            _planned.Add(source);
        }

        _plannedFor = prompt.Spell;
        _selection.Announced = Announcement.XPaidBy(prompt, ManaForX(client));
        _needsRedraw = true;
    }

    /// <summary>What a planned source wears on the board while X is announced.</summary>
    private CardLook PlannedLook(GameClient client, PayCostsPromptMessage prompt, ObjectView card)
    {
        bool planned = _planned.Contains(card.Handle);
        return new CardLook(
            Selected: planned,
            Actionable: true,
            Note: planned ? "Planned for X: click to take it back." : "Click to plan its mana for X: it is tapped as the spell is paid for.",
            Annotation: planned ? "For X" : null,
            Clicked: () => TogglePlanned(client, prompt, card.Handle));
    }

    /// <summary>
    /// Notes what a spell or ability chosen from a priority question could be paid with, for its announcement to plan, and
    /// drops any plan made before.
    /// </summary>
    private void RememberCast(PriorityPromptMessage priority, AnswerPayload answer)
    {
        _planned.Clear();
        _castMana = answer is TakeActionAnswer { Kind: ActionKindView.CastSpell or ActionKindView.ActivateAbility } chosen
            && !priority.Options.Exists(option => AutoMana.IsManaAbility(option) && option.Source == chosen.Source && option.AbilityIndex == chosen.AbilityIndex)
                ? [.. priority.Options.Where(AutoMana.IsManaAbility)]
                : [];
    }

    /// <summary>Forgets the plan once the player has priority again: the spell was cast, or given up.</summary>
    private void ForgetPlan()
    {
        _planned.Clear();
        _castMana = [];
    }

    /// <summary>
    /// Taps the next source planned for X, once the payment of what it was planned for is asked (CR 601.2g): one source an
    /// answer, by the mana ability that pays most of what is owed (<see cref="AutoMana"/>), until the pool covers the cost or
    /// the plan is spent. Whether it answered.
    /// </summary>
    private bool TapPlanned(GameClient client)
    {
        if (_planned.Count == 0 || client.Prompt?.Prompt is not PayCostsPromptMessage { AnnouncesXOrKicks: false } pay)
        {
            return false;
        }

        if (pay.Spell != _plannedFor || PoolMana(client) >= Announcement.ManaValue(pay.Remaining) || ManaOwed.Of(pay) is not { } owed)
        {
            _planned.Clear();
            return false;
        }

        // Read from the question now asked: the board's own index is the last one drawn.
        var offered = new CardActions(pay);

        // Mana already made, plain and of a type the cost still owes, is put towards it before the next source is tapped,
        // as a player paying by hand does.
        if (offered.Assignable.FirstOrDefault(option => option.Filter == ManaSpendFilterView.None
                && option.Effect == ManaSpendEffectView.None
                && option.Source.IsNone
                && option.Produces.Count > 0
                && Pays(owed, option.Produces[0])) is { } assign)
        {
            Send(client, AnswerFor(assign));
            return true;
        }

        while (_planned.Count > 0)
        {
            ViewHandle source = _planned[0];
            _planned.RemoveAt(0);

            List<ManaRow> rows = [.. CardActions.ManaRows(offered.For(source).Where(AutoMana.IsManaAbility)).Where(row => CardActions.CostPermanent(row) is null)];
            if (rows.Count == 0)
            {
                continue;
            }

            ManaRow row = rows.Count == 1 ? rows[0] : Paying(rows, offered, owed) ?? rows[0];
            Take(client, row.Option, row.Color, row.Combination, row.Sacrifice);
            return true;
        }

        return false;
    }

    /// <summary>Whether one mana of this type pays some of what is owed: a symbol of its colour, a {C}, or generic.</summary>
    private static bool Pays(ManaOwed owed, ManaTypeView mana) =>
        owed.Generic > 0
        || (mana == ManaTypeView.Colorless ? owed.Colorless > 0 : owed.CountOf((ColorView)(int)mana) > 0);

    /// <summary>The row of a planned source's that pays most of what is owed, of those that add one kind of mana.</summary>
    private static ManaRow? Paying(List<ManaRow> rows, CardActions offered, ManaOwed owed)
    {
        var uniform = rows.Where(row => row.Combination.Distinct().Count() <= 1).ToList();
        if (AutoMana.Pick(
                [.. uniform.Select(row => (row.Option, row.Color ?? (row.Combination.Count > 0 ? row.Combination[0] : null)))],
                offered.Offered,
                owed) is not { } picked)
        {
            return null;
        }

        ColorView? color = picked.Option.ProducesOfAnyColorEach == 0 && picked.Option.ColorChoices.Count > 0 ? picked.Color : null;
        List<ColorView> combination = picked.Option.ProducesOfAnyColorEach > 0 && picked.Color is { } each
            ? [.. Enumerable.Repeat(each, picked.Option.ProducesOfAnyColorEach)]
            : [];

        return uniform.FirstOrDefault(row => ReferenceEquals(row.Option, picked.Option) && row.Color == color && row.Combination.SequenceEqual(combination)) is { Option: not null } found
            ? found
            : (picked.Option, color, combination, null);
    }
}
