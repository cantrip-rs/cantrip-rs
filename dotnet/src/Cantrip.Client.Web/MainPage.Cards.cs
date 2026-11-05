using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using ManaRow = (Cantrip.Protocol.ActionOptionMessage Option, Cantrip.Protocol.ColorView? Color, System.Collections.Generic.List<Cantrip.Protocol.ColorView> Combination, Cantrip.Protocol.ViewHandle? Sacrifice);

namespace Cantrip.Client.Web;

/// <summary>
/// What each card on the table does when clicked, and how it looks while the question it answers is open, over the
/// shared <see cref="PromptSelection"/> and <see cref="CardActions"/>. A card is clickable because the outstanding prompt named it, never because this client
/// worked out that it ought to be.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>How a card is drawn and what a click on it does.</summary>
    private sealed record CardLook(
        bool Selected = false,
        bool Actionable = false,
        bool Dimmed = false,
        Action? Clicked = null,
        string? Note = null,
        string? Annotation = null);

    private static readonly CardLook Plain = new();

    private readonly PromptSelection _selection = new();
    private List<int> _picked => _selection.Cards;
    private List<int> _pickedModes => _selection.Modes;
    private List<int> _pickedColors => _selection.Colors;
    private List<TargetMessage> _pickedTargets => _selection.Targets;
    private List<int> _triggerOrder => _selection.TriggerOrder;
    private Dictionary<int, AttackChoice> _attacking => _selection.Attacking;
    private List<(int Blocker, int Attacker)> _blocking => _selection.Blocking;
    private List<int> _pending => _selection.Pending;
    private int _blockingInto { get => _selection.BlockingInto; set => _selection.BlockingInto = value; }
    private List<int> _blockQueue => _selection.BlockQueue;
    private Dictionary<(int Source, int Creature), int> _shares => _selection.DamageShares;
    private Dictionary<int, int> _dividedShares => _selection.DividedShares;
    private ActionOptionMessage? _targeting { get => _selection.Targeting; set => _selection.Targeting = value; }

    // Rebuilt from the outstanding prompt on every redraw, so no stale offer can survive into a later decision.
    private CardActions _actions = new(null);
    private readonly Dictionary<int, TargetMessage> _targetableObjects = [];
    private readonly HashSet<int> _eligible = [];
    private Dictionary<int, List<ManaRow>> _costTakes = [];

    /// <summary>Each card's look for this redraw, by handle, so a click finds what its card offered when it was drawn.</summary>
    private readonly Dictionary<ViewHandle, CardLook> _looks = [];

    private readonly CardMenu _menu = new();

    /// <summary>Where the pointer last went down, in the page's coordinates, for a card's menu to open at.</summary>
    private Windows.Foundation.Point _pointer;

    /// <summary>Whether Shift was held as the pointer went down: the Auto-mana key, held (Hotkeys: AutoMana is Shift).</summary>
    private bool _autoManaHeld;

    /// <summary>
    /// Re-indexes whatever the server is currently asking, so the board can be drawn from it.
    /// </summary>
    private void ReadPrompt(GameClient client)
    {
        _actions = new CardActions(client.Prompt?.Prompt);
        if (client.Prompt?.Prompt is PriorityPromptMessage)
        {
            ForgetPlan();
        }

        _targetableObjects.Clear();
        _eligible.Clear();
        _looks.Clear();
        _costTakes = CostCandidates();

        if (AimCandidates(client) is { } aiming)
        {
            foreach (TargetMessage candidate in aiming)
            {
                if (!candidate.IsPlayer)
                {
                    _targetableObjects[candidate.Object.Value] = candidate;
                }
            }
        }

        switch (client.Prompt?.Prompt)
        {
            case DeclareAttackersPromptMessage attackers:
                _eligible.UnionWith(attackers.Eligible.Select(handle => handle.Value));
                break;

            case DeclareBlockersPromptMessage blockers:
                _eligible.UnionWith(blockers.Eligible.Select(handle => handle.Value));
                break;

            case ChooseCardsPromptMessage choose when CardChoicePresentation.ChoiceTable(client, choose) is not null:
                _eligible.UnionWith(choose.Verdict.Addable.Select(handle => handle.Value));
                break;

            case ChooseDividedTargetsPromptMessage divide:
                _eligible.UnionWith(divide.Candidates.Where(candidate => !candidate.IsPlayer).Select(candidate => candidate.Object.Value));
                break;

            case RemoveCountersPromptMessage remove:
                _eligible.UnionWith(remove.Pools.Select(pool => pool.Object.Value));
                break;

            case AssignCombatDamagePromptMessage assign when _shares.Count == 0:
                // Seeded from the engine's own division, which is already legal, so confirming without touching it is
                // a valid answer.
                foreach (DamageDivisionMessage division in assign.Divisions)
                {
                    for (int i = 0; i < division.Creatures.Count; i++)
                    {
                        _shares[(division.Source.Value, division.Creatures[i].Value)] = i < division.Suggested.Count ? division.Suggested[i] : 0;
                    }

                    if (division.MayAssignToDefender)
                    {
                        _shares[(division.Source.Value, 0)] = division.SuggestedToDefender;
                    }
                }

                break;

            default:
                break;
        }
    }

    /// <summary>A card's look for this redraw, worked out once and remembered for its click.</summary>
    private CardLook LookOf(GameClient client, ObjectView card)
    {
        if (!_looks.TryGetValue(card.Handle, out CardLook? look))
        {
            look = Build(client, card);
            if (look.Note is null && TuckedNote(client, card) is { } note)
            {
                look = look with { Note = note };
            }

            if (look.Annotation is null && card.Zone == ZoneKindView.Battlefield && OnAnnotation(client, card) is { } on)
            {
                look = look with { Annotation = on };
            }

            if (OwnedAnnotation(client, card) is { } owned)
            {
                // Under whatever else is written over the art, a single chosen quality included, which it would hide.
                string? above = look.Annotation ?? (card.Choices.Count == 1 ? card.Choices[0].Value : null);
                look = look with { Annotation = above is null ? owned : $"{above}\n{owned}" };
            }

            if (MergedAnnotation(card) is { } merged)
            {
                look = look with { Annotation = look.Annotation is null ? merged : $"{look.Annotation}\n{merged}" };
            }

            // A card in exile or a graveyard that may be played says until when, and a flashback that it is exiled after.
            if (PermissionAnnotation(card) is { } window)
            {
                look = look with { Annotation = look.Annotation is null ? window : $"{look.Annotation}\n{window}" };
            }

            _looks[card.Handle] = look;
        }

        return look;
    }

    /// <summary>
    /// What a card in exile or a graveyard may be played for, and until when (CR 601.3, 305.1): one line for each permission
    /// it stands under. Null for a card nobody may play from where it is.
    /// </summary>
    private static string? PermissionAnnotation(ObjectView card) =>
        card.PlayPermissions.Count > 0 && card.Zone is ZoneKindView.Exile or ZoneKindView.Graveyard
            ? string.Join("\n", card.PlayPermissions.Select(permission => permission.Text))
            : null;

    /// <summary>A card placed on one of the rows, drawn as its look says.</summary>
    private CardBoardElement.BoardCell Cell(GameClient client, ObjectView card, float left, float top, bool tapped)
    {
        CardLook look = LookOf(client, card);
        bool dividing = _wornShares.TryGetValue(card.Handle, out WornShare worn);
        return new CardBoardElement.BoardCell(
            card,
            left,
            top,
            tapped,
            look.Actionable,
            // A phased-out permanent is drawn faded: the game treats it as though it does not exist (CR 702.26b).
            look.Dimmed || card.PhasedOut,
            look.Selected,
            Clickable: look.Clicked is not null,
            look.Annotation,
            DamageShare: dividing ? worn.Share : null,
            LowerLive: dividing && worn.Lower,
            RaiseLive: dividing && worn.Raise,
            ShareCounters: dividing && worn.Counters);
    }

    private void OnCardClicked(ViewHandle handle)
    {
        if (_gameClient is null || !_looks.TryGetValue(handle, out CardLook? look) || look.Clicked is not { } clicked)
        {
            return;
        }

        _tableMenu = null;
        clicked();
        RedrawNow();
    }

    /// <summary>What the outstanding prompt and the answer being built make of one card.</summary>
    private CardLook Build(GameClient client, ObjectView card)
    {
        int handle = card.Handle.Value;

        // Aiming comes first: while something is being aimed, nothing else on the board is a control.
        if (AimCandidates(client) is not null)
        {
            // A slot that judges its targets together ("with total mana value X or less", "each with a different mana
            // value"): a candidate that would break its rule beside the targets chosen is no choice now.
            bool aimable = _targetableObjects.TryGetValue(handle, out TargetMessage? target) && FitsTogether(client, target!);

            return new CardLook(
                Selected: aimable && IsPickedTarget(target!),
                Actionable: aimable,
                Dimmed: !aimable,
                Clicked: aimable ? () => Aim(client, target!) : null);
        }

        // Choosing the permanent a cost takes is aiming by another name: the candidates are lit and nothing else answers.
        if (_selection.CostChoice is not null)
        {
            bool takes = _costTakes.TryGetValue(handle, out List<ManaRow>? rows);
            return new CardLook(Actionable: takes, Dimmed: !takes, Clicked: takes ? () => PickCostPermanent(client, rows!) : null);
        }

        return client.Prompt?.Prompt switch
        {
            // A triggered ability put on the stack by a click that has not been sent yet is clicked again to take it back
            // off, and the ones put on after it stay in their order.
            OrderTriggersPromptMessage ordering when card.Zone == ZoneKindView.Stack && handle < 0 => new CardLook(
                Actionable: true,
                Note: "Click to take it back off the stack.",
                Clicked: () =>
                {
                    _triggerOrder.RemoveAll(index => TriggerStandIn(client, ordering, index).Handle.Value == handle);
                    _needsRedraw = true;
                }),

            // Discarding to hand size asks for a number of cards: once that many are picked the rest of the hand stops
            // being clickable. With one to discard, clicking another card trades rather than adding.
            DiscardPromptMessage discard when card.Zone == ZoneKindView.Hand && card.ZoneOwner == client.ActingSeat =>
                new CardLook(
                    Selected: _picked.Contains(handle),
                    Actionable: _picked.Contains(handle) || _selection.CanPick(discard.Count),
                    Clicked: _picked.Contains(handle) || _selection.CanPick(discard.Count) ? () => Pick(handle, discard.Count) : null),

            // A choice among cards already in view is made where they are; the rest of that zone is dimmed.
            ChooseCardsPromptMessage choose when CardChoicePresentation.ChoiceTable(client, choose) is { } table && InTable(client, card, table) =>
                new CardLook(
                    Selected: _picked.Contains(handle),
                    Dimmed: !_eligible.Contains(handle),
                    Actionable: _eligible.Contains(handle) && !_picked.Contains(handle) && _selection.CanPick(choose.Count)
                        && CardChoicePresentation.Fits(client, choose, _picked, handle),
                    Clicked: _eligible.Contains(handle) && (_picked.Contains(handle) || _selection.CanPick(choose.Count) && CardChoicePresentation.Fits(client, choose, _picked, handle))
                        ? () => PickOnBoard(client, choose, handle)
                        : null,
                    Annotation: PutBackAnnotation(choose, handle)),

            DeclareAttackersPromptMessage attackers when _eligible.Contains(handle) =>
                new CardLook(
                    Selected: _attacking.ContainsKey(handle) || _pending.Contains(handle),
                    Actionable: true,
                    Clicked: () => ClickAttacker(attackers, handle)),

            // With creatures picked up, the planeswalkers and battles one of them may attack are what to click.
            DeclareAttackersPromptMessage attackers
                when _pending.Count > 0 && _pending.Any(creature => MayAttack(attackers, creature, new AttackChoice(0, card.Handle))) =>
                new CardLook(Actionable: true, Clicked: () => AssignAttack(client, attackers, new AttackChoice(0, card.Handle))),

            // A creature is picked up, and the attackers become the things to click. One already blocking is picked up again
            // where the rules' verdict lets it block another attacker too, and otherwise comes back off what it blocks.
            // A creature still on its way to the attacker clicked waits for the host's verdict, and is no control meanwhile.
            DeclareBlockersPromptMessage when _eligible.Contains(handle) && _blockQueue.Contains(handle) =>
                new CardLook(Selected: true),

            DeclareBlockersPromptMessage when _eligible.Contains(handle) =>
                new CardLook(
                    Selected: _pending.Contains(handle) || _blocking.Exists(pair => pair.Blocker == handle),
                    Actionable: true,
                    Clicked: () => ClickBlocker(client, handle)),

            DeclareBlockersPromptMessage blockers
                when _pending.Count > 0
                    && blockers.Attackers.Contains(card.Handle)
                    && _pending.Any(blocker => MayBlock(client, blocker, card.Handle.Value)) =>
                new CardLook(Actionable: true, Clicked: () => AssignBlocks(client, card.Handle)),

            // An attacker none of the held blockers may block, such as a flier against creatures without flying or reach.
            DeclareBlockersPromptMessage blockers
                when _pending.Count > 0 && blockers.Attackers.Contains(card.Handle) =>
                new CardLook(Dimmed: true, Note: "Can't be blocked by " + (_pending.Count == 1 ? "that creature." : "those creatures.")),

            // A mana source while X is announced (CR 601.2b): planned for X, to be tapped as the spell is paid for (CR 601.2g).
            PayCostsPromptMessage plan when PlansMana(plan) && card.Zone == ZoneKindView.Battlefield && ManaFrom(card.Handle) > 0 =>
                PlannedLook(client, plan, card),

            // A spell with assist while mana is made for it and the help is announced (CR 702.132a): nothing is paid yet, and
            // the question's Remaining is the whole cost.
            PayCostsPromptMessage { AssistStep: AssistSteps.HelperActivates or AssistSteps.CasterActivates or AssistSteps.HelperAnnounces } assisted
                when assisted.Spell == card.Handle && _actions.For(card.Handle).Count == 0 =>
                new CardLook(Note: assisted.AssistStep == AssistSteps.CasterActivates
                    ? $"Being cast with assist. It costs {assisted.Total}."
                    : $"Being cast with assist. Its generic mana is {assisted.Total}."),

            // The spell being paid for. Not a control, but it says what it is waiting on - unless it can help pay.
            PayCostsPromptMessage pay when pay.Spell == card.Handle && _actions.For(card.Handle).Count == 0 =>
                new CardLook(Note: card.Zone switch
                {
                    ZoneKindView.Battlefield => $"Being activated. {pay.Remaining} still to pay.",
                    ZoneKindView.Stack => $"{pay.Remaining} still to pay.",
                    _ => $"Being cast. {pay.Remaining} still to pay.",
                }),

            // A land offered while paying is clicked exactly as it is from a priority prompt.
            PriorityPromptMessage or PayCostsPromptMessage or UnlessPaysPromptMessage
                when _actions.For(card.Handle) is { Count: > 0 } offered =>
                new CardLook(Actionable: true, Clicked: () => Choose(client, offered)),

            DeclareAttackersPromptMessage or DeclareBlockersPromptMessage =>
                new CardLook(Dimmed: card.Controller == client.ActingSeat && card.CardTypes.Contains(CardTypeView.Creature)),

            // A permanent whose counters a cost may take (CR 118.3): a click takes one more of them, and it says how many are taken.
            RemoveCountersPromptMessage remove when remove.Pools.Any(pool => pool.Object == card.Handle) =>
                new CardLook(
                    Selected: remove.Pools.Any(pool => pool.Object == card.Handle && _selection.CounterRemovals.GetValueOrDefault((handle, pool.Counter)) > 0),
                    Actionable: true,
                    Note: CountersTakenNote(remove, card.Handle),
                    Clicked: () => TakeCounterFrom(remove, card.Handle)),

            RemoveCountersPromptMessage when card.Zone == ZoneKindView.Battlefield => new CardLook(Dimmed: true),

            // A divided slot's candidates wear their shares (CR 601.2d); a click on one adds to its share, as its + does.
            ChooseDividedTargetsPromptMessage divide when divide.Candidates.Any(candidate => !candidate.IsPlayer && candidate.Object == card.Handle) =>
                new CardLook(
                    Selected: _dividedShares.GetValueOrDefault(handle) > 0,
                    Actionable: true,
                    Clicked: () => OnShareStepped(card.Handle, raise: true)),

            ChooseDividedTargetsPromptMessage when card.Zone == ZoneKindView.Battlefield => new CardLook(Dimmed: true),

            _ => Plain,
        };
    }

    private static bool InTable(GameClient client, ObjectView card, ZoneKindView table) =>
        card.Zone == table && (table != ZoneKindView.Hand || card.ZoneOwner == client.ActingSeat);

    private string? PutBackAnnotation(ChooseCardsPromptMessage prompt, int handle) =>
        prompt.Purpose is CardChoicePurposeView.PutOnTop or CardChoicePurposeView.Splice
            && _picked.IndexOf(handle) is >= 0 and var at
            ? prompt.Purpose == CardChoicePurposeView.Splice ? $"Effect {at + 1}"
                : at == prompt.Count - 1 ? "On top" : $"{at + 1}"
            // A choice "with total mana value N or less" (CR 202.3): what each card it may take counts, as the engine read it
            // (an {X} in a library is 0, CR 202.3e).
            : prompt.MaximumTotalManaValue is not null && prompt.Cards.FindIndex(card => card.Value == handle) is >= 0 and var index
                && index < prompt.ManaValues.Count
                ? $"Mana value {prompt.ManaValues[index]}"
            : null;

    /// <summary>
    /// Acts on a card. One thing to offer does it; more than one opens the menu. An ability that asks
    /// nothing opens the menu even alone, since nothing else stands between the click and the stack.
    /// </summary>
    private void Choose(GameClient client, IReadOnlyList<ActionOptionMessage> offered)
    {
        if (client.Prompt is null)
        {
            return;
        }

        List<ManaRow> rows = CardActions.ManaRows(offered);
        var entries = CardActions.MenuEntries(rows);

        // One thing to do, and it wants a permanent: straight to picking it, whatever auto-mana would say.
        if (entries.Count == 1 && CardActions.CostPermanent(entries[0][0]) is not null)
        {
            ChooseCostPermanent(entries[0]);
            return;
        }

        // Taking a companion (CR 116.2g) is offered as its menu row, "{3}: Put [card] in hand.", even alone: the card in the
        // companion window says nothing else of what a click on it does.
        if (rows.Count == 1 && rows[0].Option.Kind != ActionKindView.TakeCompanion
            && (!CardActions.IsIrreversibleActivation(rows[0].Option) || CardActions.AsksBeforeActivating(rows[0].Option)))
        {
            Take(client, rows[0].Option, rows[0].Color, rows[0].Combination, rows[0].Sacrifice);
            return;
        }

        if (AutoManaPick(client, rows) is { } picked)
        {
            if (CardActions.CostPermanent(picked) is not null)
            {
                ChooseCostPermanent(entries.First(entry => entry.Any(row => CardActions.SameAbility(row.Option, picked.Option))));
            }
            else
            {
                Take(client, picked.Option, picked.Color, picked.Combination, picked.Sacrifice);
            }

            return;
        }

        OpenCardMenu(client, offered[0].Source, MenuItems(client, entries), [.. entries.Select(entry => entry[0].Option.AbilityIndex)], [.. entries.Select(entry => entry[0].Option.OptionId)]);
    }

    /// <summary>A card's menu rows, each worded and with what choosing it does: one for each thing it offers.</summary>
    private List<(string Text, Action OnChosen)> MenuItems(GameClient client, List<List<ManaRow>> entries)
    {
        List<(string, Action)> items = [];
        foreach (var entry in entries)
        {
            (ActionOptionMessage option, ColorView? color, List<ColorView> combination, ViewHandle? sacrifice) = entry[0];
            items.Add(CardActions.CostPermanent(entry[0]) is not null
                ? (CostChoiceLabel(client, entry), () => { ChooseCostPermanent(entry); RedrawNow(); })
                : (CardActions.Describe(client, option, color, combination, Sacrificed(client, sacrifice)),
                    () => { Take(client, option, color, combination, sacrifice); RedrawNow(); }));
        }

        return items;
    }

    /// <summary>Whether auto-mana answers a card's menu: on by default or not, and Shift held turns it the other way.</summary>
    private bool AutoManaWanted() => PlayerSettings.AutoManaByDefault != _autoManaHeld;

    /// <summary>What auto-mana would take from these rows, or null when it is not wanted, nothing is owed, or it has no pick.</summary>
    private ManaRow? AutoManaPick(GameClient client, List<ManaRow> rows)
    {
        if (!AutoManaWanted() || ManaOwed.Of(client.Prompt?.Prompt) is not { Any: true } owed)
        {
            return null;
        }

        var uniform = rows.Where(row => row.Combination.Distinct().Count() <= 1).ToList();
        if (AutoMana.Pick(
                [.. uniform.Select(row => (row.Option, row.Color ?? (row.Combination.Count > 0 ? row.Combination[0] : null)))],
                _actions.Offered,
                owed) is not { } picked)
        {
            return null;
        }

        ColorView? color = picked.Option.ProducesOfAnyColorEach == 0 && picked.Option.ColorChoices.Count > 0 ? picked.Color : null;
        List<ColorView> combination = picked.Option.ProducesOfAnyColorEach > 0 && picked.Color is { } each
            ? [.. Enumerable.Repeat(each, picked.Option.ProducesOfAnyColorEach)]
            : [];

        foreach (var row in uniform)
        {
            if (ReferenceEquals(row.Option, picked.Option) && row.Color == color && row.Combination.SequenceEqual(combination))
            {
                return row;
            }
        }

        return (picked.Option, color, combination, null);
    }

    private void ChooseCostPermanent(List<ManaRow> entry)
    {
        _selection.CostChoice = [.. entry.Select(row => row.Option).Distinct()];
        _needsRedraw = true;
    }

    private Dictionary<int, List<ManaRow>> CostCandidates()
    {
        Dictionary<int, List<ManaRow>> candidates = [];
        if (_selection.CostChoice is not { } options)
        {
            return candidates;
        }

        foreach (var row in CardActions.ManaRows(options))
        {
            if (CardActions.CostPermanent(row) is { } permanent)
            {
                if (!candidates.TryGetValue(permanent.Value, out var taking))
                {
                    candidates[permanent.Value] = taking = [];
                }

                taking.Add(row);
            }
        }

        return candidates;
    }

    private void PickCostPermanent(GameClient client, List<ManaRow> rows)
    {
        _selection.CostChoice = null;
        _needsRedraw = true;

        if (rows.Count == 1)
        {
            Take(client, rows[0].Option, rows[0].Color, rows[0].Combination, rows[0].Sacrifice);
            return;
        }

        if (AutoManaPick(client, rows) is { } picked)
        {
            Take(client, picked.Option, picked.Color, picked.Combination, picked.Sacrifice);
            return;
        }

        List<(string, Action)> items = [];
        foreach ((ActionOptionMessage option, ColorView? color, List<ColorView> combination, ViewHandle? sacrifice) in rows)
        {
            items.Add((
                CardActions.Describe(client, option, color, combination, Sacrificed(client, sacrifice)),
                () => { Take(client, option, color, combination, sacrifice); RedrawNow(); }));
        }

        OpenCardMenu(client, rows[0].Option.Source, items, [.. rows.Select(row => row.Option.AbilityIndex)], [.. rows.Select(row => row.Option.OptionId)]);
    }

    private static string CostChoiceLabel(GameClient client, List<ManaRow> entry)
    {
        List<string> readings =
            [.. entry.Select(row => CardActions.Describe(client, row.Option, row.Color, row.Combination, anyPermanent: true)).Distinct()];
        return readings.Count == 1 ? readings[0] : $"{entry[0].Option.CostText}: Add mana...";
    }

    /// <summary>Takes an action, first asking which target when the server offered a choice of them.</summary>
    private void Take(
        GameClient client,
        ActionOptionMessage option,
        ColorView? color = null,
        IReadOnlyList<ColorView>? combination = null,
        ViewHandle? sacrifice = null)
    {
        // The colour a mana row named is kept while the permanents its cost taps are picked (Supportive Parents).
        if (CardActions.NeedsATarget(option))
        {
            _targeting = option;
            _selection.TargetingColor = color;
            _selection.TargetingColors.Clear();
            _selection.TargetingColors.AddRange(combination ?? []);
            PeekAtTargets(client, option.TargetCandidates);
            _needsRedraw = true;
            return;
        }

        TakeActionAnswer answer = Taking(option, color, combination);
        answer.Sacrifice = sacrifice;
        Send(client, answer);
    }

    /// <summary>An action taken, with the colour or colours chosen for the mana it adds.</summary>
    private static TakeActionAnswer Taking(ActionOptionMessage option, ColorView? color, IReadOnlyList<ColorView>? combination)
    {
        TakeActionAnswer answer = AnswerFor(option);
        answer.Color = color;
        answer.Colors = [.. combination ?? []];

        // Colorless is not a colour (CR 105.1), so a choice of colorless alone is answered as the mana type instead.
        if (CardActions.ChoosesColorless(option))
        {
            answer.Mana = ManaTypeView.Colorless;
        }

        return answer;
    }

    private static string Sacrificed(GameClient client, ViewHandle? sacrifice) =>
        sacrifice is { } handle ? client.Object(handle)?.Name ?? string.Empty : string.Empty;

    /// <summary>What can be aimed at right now: a spell being announced, or a triggered ability's target prompt.</summary>
    private IReadOnlyList<TargetMessage>? AimCandidates(GameClient client) =>
        _targeting?.TargetCandidates
        ?? (client.Prompt?.Prompt as ChooseTargetPromptMessage)?.Candidates;

    private void PickOnBoard(GameClient client, ChooseCardsPromptMessage choose, int handle)
    {
        Pick(handle, choose.Count);

        // The click that reaches an exact number of permanents answers, once the host calls the cards picked an answer.
        CardChoicePresentation model = CardChoicePresentation.Describe(client, choose, _picked);
        if (model.AnswersOnLastPick && _picked.Count == choose.Count)
        {
            if (model.CanSubmit)
            {
                Send(client, new CardsChosenAnswer { Cards = [.. _picked.Select(h => new ViewHandle(h))] });
            }
            else
            {
                _answerWhenJudged = true;
            }
        }
    }

    /// <summary>
    /// Aims at a target. An exact number answers itself on the click that reaches it; "up to" a number
    /// collects clicks until the dock's button. Clicking a picked one puts it back; with one to choose, another trades.
    /// </summary>
    private void Aim(GameClient client, TargetMessage target)
    {
        (int count, bool upTo) = _targeting is { } announced
            ? (Math.Max(announced.TargetsRequired, 1), announced.TargetsOptional)
            : client.Prompt?.Prompt is ChooseTargetPromptMessage asked ? (asked.Count, asked.Optional || asked.Minimum > 0) : (1, false);

        if (_pickedTargets.RemoveAll(picked => Same(picked, target)) == 0)
        {
            if (count == 1)
            {
                _pickedTargets.Clear();
            }

            // A target the slot's rule of its targets together would refuse beside the others is not added (CR 601.2c).
            if (_pickedTargets.Count < count && FitsTogether(client, target))
            {
                _pickedTargets.Add(target);
            }
        }

        if (upTo || _pickedTargets.Count < count)
        {
            _needsRedraw = true;
            return;
        }

        SendTargets(client);
    }

    private void SendTargets(GameClient client)
    {
        if (_targeting is { } announced)
        {
            TakeActionAnswer answer = Taking(announced, _selection.TargetingColor, _selection.TargetingColors);
            answer.Targets = [.. _pickedTargets];
            Send(client, answer);
            return;
        }

        Send(client, new TargetsChosenAnswer { Targets = [.. _pickedTargets] });
    }

    /// <summary>
    /// Whether <paramref name="target"/> may be chosen beside the targets already picked: a slot "with total mana value X or
    /// less" (Fire Lord Sozin's) takes no target that would go over it, and a slot whose rule of its targets together the
    /// host judges (<see cref="ChooseTargetPromptMessage.Verdict"/>: Agadeem's Awakening's "each have a different mana value
    /// X or less") only an object its verdict on the objects picked so far says may be added - asked of the host the first
    /// time those are picked, and nothing offered until it answers (with one to choose, a click trades, so the object is
    /// judged alone). A player is no card to the rule, and one already picked may always be clicked (to put it back).
    /// </summary>
    private bool FitsTogether(GameClient client, TargetMessage target)
    {
        if (_targeting is not null || client.Prompt?.Prompt is not ChooseTargetPromptMessage asked || IsPickedTarget(target))
        {
            return true;
        }

        if (asked.MaximumTotalManaValue is not null && !asked.Fits(_pickedTargets, target))
        {
            return false;
        }

        if (asked.Verdict is null || target.IsPlayer)
        {
            return true;
        }

        List<int> picked = [.. _pickedTargets.Where(chosen => !chosen.IsPlayer).Select(chosen => chosen.Object.Value)];
        SelectionVerdictView? verdict = asked.Count == 1 ? asked.Verdict : CardChoicePresentation.Judged(client, asked, picked);
        return verdict?.Addable.Contains(target.Object) == true;
    }

    private static bool Same(TargetMessage one, TargetMessage other) =>
        one.Player == other.Player && one.Object == other.Object;

    private bool IsPickedTarget(TargetMessage candidate) => _pickedTargets.Any(picked => Same(picked, candidate));

    /// <summary>
    /// Whether what is being paid for returns a tapped creature (web-slinging, CR 702.188a; Urban Retreat): it is chosen once the mana is
    /// paid, so the payment's caption says to tap a creature for mana while none is tapped, or the cast is taken back.
    /// </summary>
    private bool _tappedAfterMana;

    private static TakeActionAnswer AnswerFor(ActionOptionMessage option) => new()
    {
        Kind = option.Kind,
        Source = option.Source,
        AbilityIndex = option.AbilityIndex,
        OptionId = option.OptionId,
        Tap = option.Tap,
        Mana = option.Kind is ActionKindView.AssignMana or ActionKindView.PayLifeForSymbol ? option.Produces[0] : null,
        Filter = option.Filter,
        Effect = option.Effect,
        SpendSubtypes = option.SpendSubtypes,
        SpendManaValue = option.SpendManaValue,
        Snow = option.Snow,
        PaySnow = option.PaySnow,
    };

    private void Send(GameClient client, AnswerPayload answer)
    {
        if (client.Prompt is null)
        {
            return;
        }

        // Whether the cast or activation just chosen returns a tapped creature (CR 702.188a, 601.2g-h), for the payment's caption to say what it needs.
        if (client.Prompt.Prompt is PriorityPromptMessage priority)
        {
            RememberCast(priority, answer);
            _tappedAfterMana = answer is TakeActionAnswer taken && priority.Options.Exists(option => AutoPlay.ReturnsTappedAfterMana(option)
                && option.Source == taken.Source && option.AbilityIndex == taken.AbilityIndex && option.OptionId == taken.OptionId && option.Kind == taken.Kind);
        }

        client.Answer(answer);
        _selection.Clear();
        _needsRedraw = true;
    }

    private void Pick(int handle, int limit)
    {
        _selection.Pick(handle, limit);
        _needsRedraw = true;
    }

    /// <summary>
    /// Cycles an attacker through not attacking, then each player it could attack in turn, then each planeswalker they
    /// control (CR 508.1b).
    /// </summary>
    private void CycleAttack(int handle, List<AttackChoice> defenders)
    {
        if (!_attacking.TryGetValue(handle, out AttackChoice defender))
        {
            _attacking[handle] = defenders[0];
        }
        else
        {
            int next = defenders.IndexOf(defender) + 1;

            if (next >= defenders.Count)
            {
                _attacking.Remove(handle);
            }
            else
            {
                _attacking[handle] = defenders[next];
            }
        }

        _needsRedraw = true;
    }

    /// <summary>What one creature may be sent at: every offered seat and planeswalker but those closed to it (CR 508.1c).</summary>
    /// <summary>
    /// What <paramref name="attacker"/> may be sent at: the attacks the host's verdict on no attacks offers it (CR 508.1c-d) -
    /// for a creature an effect requires to attack a certain player, only those obeying as much - seats first.
    /// </summary>
    private static List<AttackChoice> AttackChoices(DeclareAttackersPromptMessage prompt, ViewHandle attacker) =>
        [.. prompt.OfferedFor(attacker).Select(attack => new AttackChoice(attack.TargetPlayer, attack.TargetPermanent))];

    /// <summary>Picks a creature up, or puts it back down, to be sent somewhere along with the others picked up.</summary>
    private void PickUp(int creature)
    {
        // A creature picked up goes where the next click sends it, not to the attacker the last one did.
        if (_blockQueue.Count == 0)
        {
            _blockingInto = 0;
        }
        if (!_pending.Remove(creature))
        {
            _pending.Add(creature);
        }

        _needsRedraw = true;
    }

    /// <summary>Whether a creature may be declared making an attack (CR 508.1c).</summary>
    private static bool MayAttack(DeclareAttackersPromptMessage prompt, int attacker, AttackChoice choice) =>
        prompt.OfferedFor(new ViewHandle(attacker)).Exists(attack => attack.TargetPlayer == choice.Player && attack.TargetPermanent == choice.Permanent);

    /// <summary>The blocks declared so far, as the answer and the host's check name them.</summary>
    private List<BlockMessage> DeclaredBlocks() =>
        [.. _blocking.Select(pair => new BlockMessage { Blocker = new ViewHandle(pair.Blocker), Attacker = new ViewHandle(pair.Attacker) })];

    /// <summary>
    /// What the rules make of the blocks declared so far (CR 509.1a-c): the host's verdict, asked the first time they are
    /// declared and null until it comes back.
    /// </summary>
    private BlockVerdictView? DeclaredVerdict(GameClient client)
    {
        List<BlockMessage> declared = DeclaredBlocks();
        BlockVerdictView? verdict = client.BlockVerdict(declared);
        if (verdict is null)
        {
            client.CheckBlocks(declared);
        }

        return verdict;
    }

    /// <summary>
    /// Whether a click may have <paramref name="blocker"/> block <paramref name="attacker"/>: the host's verdict on the blocks
    /// so far offers the block, or it is made already and the click takes it back. Nothing is offered until the verdict is in.
    /// </summary>
    private bool MayBlock(GameClient client, int blocker, int attacker) =>
        _blocking.Contains((blocker, attacker))
        || DeclaredVerdict(client)?.Addable.Exists(pair => pair.Blocker.Value == blocker && pair.Attacker.Value == attacker) == true;

    /// <summary>
    /// Whether the attackers have a choice of what to attack: more than one seat, planeswalker or battle that some
    /// creature may be sent at. With none, a click on a creature is all it takes (<see cref="CycleAttack"/>).
    /// </summary>
    private static bool ChoosesWhatToAttack(DeclareAttackersPromptMessage prompt) =>
        prompt.Eligible.Any(creature => prompt.AttacksFor(creature).Count > 1);

    /// <summary>
    /// A click on a creature able to attack. Where there is one thing to attack the creature is sent at it, and a
    /// second click takes it back; where there are several it is picked up with the others and the click on what to
    /// attack declares them all.
    /// </summary>
    private void ClickAttacker(DeclareAttackersPromptMessage prompt, int handle)
    {
        if (!ChoosesWhatToAttack(prompt))
        {
            CycleAttack(handle, AttackChoices(prompt, new ViewHandle(handle)));
        }
        else if (_attacking.Remove(handle))
        {
            _needsRedraw = true;
        }
        else
        {
            PickUp(handle);
        }
    }

    /// <summary>Sends every creature picked up that may at the chosen seat, planeswalker or battle, and the arrows to it go in.</summary>
    private void AssignAttack(GameClient client, DeclareAttackersPromptMessage prompt, AttackChoice choice)
    {
        List<int> sent = [.. _pending.Where(creature => MayAttack(prompt, creature, choice))];
        RetireArrows(sent, AttackBounds(client, choice));

        foreach (int creature in sent)
        {
            _attacking[creature] = choice;
            _pending.Remove(creature);
        }

        _needsRedraw = true;
    }

    /// <summary>
    /// Sends the creatures picked up at <paramref name="attacker"/>: a creature picked up that blocks it already comes back off
    /// it, and the others go to it one at a time, each as the host's verdict on the blocks so far offers it
    /// (<see cref="ContinueBlocking"/>) - so an attacker that can't be blocked by more than one creature (CR 509.1b) takes one,
    /// and the rest stay picked up while the dock says why.
    /// </summary>
    private void AssignBlocks(GameClient client, ViewHandle attacker)
    {
        int target = attacker.Value;
        foreach (int blocker in _pending.Where(blocker => _blocking.Contains((blocker, target))).ToList())
        {
            _blocking.Remove((blocker, target));
            _pending.Remove(blocker);
        }

        _blockingInto = target;
        _blockQueue.Clear();
        _blockQueue.AddRange(_pending);
        _pending.Clear();
        ContinueBlocking(client);
        _needsRedraw = true;
    }

    /// <summary>
    /// Takes the creatures picked up to the attacker last clicked as far as the verdicts in hand allow: each that the verdict
    /// on the blocks so far offers blocks it, its arrow going in, and the next waits for the host's verdict on the blocks with
    /// it. Ends where none picked up is offered; those stay picked up.
    /// </summary>
    private void ContinueBlocking(GameClient client)
    {
        while (_blockingInto != 0 && _blockQueue.Count > 0)
        {
            BlockVerdictView? verdict = DeclaredVerdict(client);
            if (verdict is null)
            {
                return;
            }

            int target = _blockingInto;
            int? next = _blockQueue.Where(blocker => verdict.Addable.Exists(pair => pair.Blocker.Value == blocker && pair.Attacker.Value == target))
                .Select(blocker => (int?)blocker)
                .FirstOrDefault();
            if (next is not { } blocker)
            {
                // The attacker takes none of the rest: they are picked up again, and the dock says why.
                _pending.AddRange(_blockQueue.Where(creature => !_pending.Contains(creature)));
                _blockQueue.Clear();
                _needsRedraw = true;
                return;
            }

            RetireArrows([blocker], CardBounds(new ViewHandle(target)));
            _blocking.Add((blocker, target));
            _blockQueue.Remove(blocker);
            _needsRedraw = true;
        }

        if (_blockQueue.Count == 0 && _pending.Count == 0)
        {
            _blockingInto = 0;
        }
    }

    /// <summary>
    /// A click on a creature able to block: one picked up is put back down; one blocking that the verdict lets block another
    /// attacker too is picked up again, and one that may block nothing more comes back off what it blocks; any other is picked up.
    /// </summary>
    private void ClickBlocker(GameClient client, int blocker)
    {
        // One on its way to an attacker is not picked up again: the host's verdict sends it there, or leaves it picked up.
        if (_blockQueue.Contains(blocker))
        {
            return;
        }

        if (_pending.Remove(blocker))
        {
            _needsRedraw = true;
            return;
        }

        if (_blocking.Exists(pair => pair.Blocker == blocker)
            && DeclaredVerdict(client)?.Addable.Exists(pair => pair.Blocker.Value == blocker) != true)
        {
            _blocking.RemoveAll(pair => pair.Blocker == blocker);
            _needsRedraw = true;
            return;
        }

        PickUp(blocker);
    }

    /// <summary>Puts down whatever was picked up.</summary>
    private void PutDown()
    {
        _pending.Clear();
        if (_blockQueue.Count == 0)
        {
            _blockingInto = 0;
        }
        _needsRedraw = true;
    }

    /// <summary>
    /// Whether a land has been spent: tapped, and nothing this client is offering or asking about it,
    /// so it joins the one pile of spent lands.
    /// </summary>
    private bool IsSpent(ObjectView land)
    {
        int handle = land.Handle.Value;

        return land.Tapped
            && TuckedLikeness(land).Length == 0
            && _actions.For(land.Handle).Count == 0
            && !_eligible.Contains(handle)
            && !_targetableObjects.ContainsKey(handle)
            && !_picked.Contains(handle);
    }

    /// <summary>
    /// The cards in exile linked to a permanent (CR 607.2a): those it holds "until it leaves the battlefield" (CR 610.3)
    /// and those exiled with it for good.
    /// </summary>
    private static IEnumerable<ObjectView> HeldBy(GameClient client, ViewHandle holder) => Holdings(client).Held[holder];

    /// <summary>The permanents attached to this one: the Auras enchanting it and the Equipment attached to it (CR 301.5c, 303.4).</summary>
    private static IEnumerable<ObjectView> WornBy(GameClient client, ViewHandle host) => Holdings(client).Worn[host];

    private static (PlayerView? View, ILookup<ViewHandle, ObjectView> Held, ILookup<ViewHandle, ObjectView> Worn) _holdings;

    /// <summary>
    /// What each permanent holds in exile and has attached to it, gathered once per view: every permanent on a redraw
    /// asks, and a pass over the zone per ask made a board of two hundred permanents cost forty thousand looks a redraw.
    /// </summary>
    private static (PlayerView? View, ILookup<ViewHandle, ObjectView> Held, ILookup<ViewHandle, ObjectView> Worn) Holdings(GameClient client)
    {
        if (!ReferenceEquals(_holdings.View, client.View) || _holdings.Held is null)
        {
            _holdings = (
                client.View,
                client.Zone(ZoneKindView.Exile, 0).Where(card => card.ExiledWith != ViewHandle.None).ToLookup(card => card.ExiledWith),
                client.Battlefield().Where(card => card.AttachedTo != ViewHandle.None).ToLookup(card => card.AttachedTo));
        }

        return _holdings;
    }

    /// <summary>
    /// Whether this permanent is drawn under the one it is attached to rather than standing in a row of its own. An Aura on a card in a graveyard has no permanent to be drawn under, so it keeps its place.
    /// </summary>
    private static bool Nested(GameClient client, ObjectView card) =>
        card.AttachedTo != ViewHandle.None
        && client.Object(card.AttachedTo) is { Zone: ZoneKindView.Battlefield };

    /// <summary>
    /// Everything tucked under a permanent, furthest back first: the cards it holds in exile, then
    /// each thing attached to it after whatever that holds in turn.
    /// </summary>
    private static List<ObjectView> Tucked(GameClient client, ObjectView holder)
    {
        List<ObjectView> under = [.. HeldBy(client, holder.Handle)];
        foreach (ObjectView worn in WornBy(client, holder.Handle))
        {
            under.AddRange(Tucked(client, worn));
            under.Add(worn);
        }

        return under;
    }

    /// <summary>What a tucked card says about where it is, when its prompt has nothing to say.</summary>
    private static string? TuckedNote(GameClient client, ObjectView card)
    {
        if (Nested(client, card) && client.Object(card.AttachedTo) is { } host)
        {
            return $"Attached to {host.Name}.";
        }

        if (card.Zone == ZoneKindView.Exile && card.ExiledWith != ViewHandle.None
            && client.Object(card.ExiledWith) is { Zone: ZoneKindView.Battlefield } holder)
        {
            return card.Kind == ObjectKindView.CardCopy
                ? $"Prepared by {holder.Name}. Cast it from here for as long as {holder.Name} stays prepared on the battlefield."
                : card.ExiledUntilLeaves
                    ? $"Exiled by {holder.Name} until it leaves the battlefield."
                    : $"Exiled with {holder.Name}.";
        }

        return null;
    }

    /// <summary>
    /// What an attachment standing in a row of its own says it is on: the card it enchants when that
    /// is not a permanent, or the player.
    /// </summary>
    private static string? OnAnnotation(GameClient client, ObjectView card)
    {
        if (card.AttachedTo != ViewHandle.None && !Nested(client, card) && client.Object(card.AttachedTo) is { } host)
        {
            return $"On {host.Name}";
        }

        if (card.AttachedToPlayer != 0)
        {
            PlayerSummaryView? player = client.View?.Players.FirstOrDefault(seat => seat.Seat == card.AttachedToPlayer);
            return $"On {player?.Name ?? $"Seat {card.AttachedToPlayer}"}";
        }

        // CR 310.9: a battle's protector, who defends it and alone may not attack it.
        if (card.Protector != 0)
        {
            return card.Protector == client.Seat ? "You protect it" : $"Protected by {NameOf(client, card.Protector)}";
        }

        return null;
    }

    /// <summary>
    /// What marks a merged permanent (CR 730.2), written over its art: it is one permanent of several cards, the topmost's
    /// face drawn, and the other cards named, topmost first (a face-down one the viewer may not look at is "a face-down card").
    /// Null for anything else.
    /// </summary>
    private static string? MergedAnnotation(ObjectView card) =>
        card.Zone == ZoneKindView.Battlefield && card.MergedWith.Count > 0
            ? $"Merged with {string.Join(", ", card.MergedWith.Select(part => part.Known ? part.Name : "a face-down card"))}"
            : null;

    /// <summary>Who owns a permanent or spell its controller does not (CR 108.3), or null for one controlled by its owner.</summary>
    private static string? OwnedAnnotation(GameClient client, ObjectView card)
    {
        if (card.Zone is not (ZoneKindView.Battlefield or ZoneKindView.Stack) || card.Owner == 0 || card.Owner == card.Controller)
        {
            return null;
        }

        if (card.Owner == client.Seat)
        {
            return "Owned by you";
        }

        PlayerSummaryView? owner = client.View?.Players.FirstOrDefault(seat => seat.Seat == card.Owner);
        return $"Owned by {owner?.Name ?? $"Seat {card.Owner}"}";
    }

    /// <summary>
    /// The handles of what is tucked under a permanent, so one holding something never piles with a twin that holds
    /// nothing, or something else. Empty for one holding nothing.
    /// </summary>
    private string TuckedLikeness(ObjectView card) =>
        _gameClient is null || card.Zone != ZoneKindView.Battlefield
            ? string.Empty
            : string.Join(",", Tucked(_gameClient, card).Select(under => under.Handle.Value));

    /// <summary>What the outstanding prompt and the half-made answer say about a card, for telling twins apart.</summary>
    private string PromptLikeness(ObjectView card)
    {
        int handle = card.Handle.Value;
        return string.Concat(
            TuckedLikeness(card),
            "|",
            _eligible.Contains(handle) ? "e" : "-",
            _targetableObjects.TryGetValue(handle, out TargetMessage? aimed) ? IsPickedTarget(aimed) ? "T" : "t" : "-",
            _selection.CostChoice is null ? "-" : _costTakes.ContainsKey(handle) ? "c" : "x",
            _picked.Contains(handle) ? "p" : "-",
            _pending.Contains(handle) ? "a" : "-",
            _planned.Contains(card.Handle) ? "x" : "-",
            string.Join(',', _blocking.Where(pair => pair.Blocker == handle).Select(pair => pair.Attacker.ToString(System.Globalization.CultureInfo.InvariantCulture))),
            "|",
            (_shares.Where(pair => pair.Key.Creature == handle).Sum(pair => pair.Value) + _dividedShares.GetValueOrDefault(handle)).ToString(System.Globalization.CultureInfo.InvariantCulture),
            "|",
            _selection.CounterRemovals.Where(pair => pair.Key.Object == handle).Sum(pair => pair.Value).ToString(System.Globalization.CultureInfo.InvariantCulture),
            "|",
            _actions.For(card.Handle).Count.ToString(System.Globalization.CultureInfo.InvariantCulture));
    }

    /// <summary>
    /// What a permanent says while a cost removes counters from among permanents: how many of each kind of its counters are taken,
    /// "2 of 2 -1/-1 counters taken", or that a click takes one.
    /// </summary>
    private string CountersTakenNote(RemoveCountersPromptMessage prompt, ViewHandle permanent)
    {
        List<string> taken =
        [
            .. prompt.Pools
                .Where(pool => pool.Object == permanent)
                .Select(pool => (pool, n: _selection.CounterRemovals.GetValueOrDefault((pool.Object.Value, pool.Counter))))
                .Where(chosen => chosen.n > 0)
                .Select(chosen => $"{chosen.n} of {chosen.pool.Count} {chosen.pool.Counter} counter{(chosen.pool.Count == 1 ? string.Empty : "s")} taken"),
        ];
        return taken.Count > 0 ? string.Join("; ", taken) + "." : "Click to take one of its counters.";
    }
}
