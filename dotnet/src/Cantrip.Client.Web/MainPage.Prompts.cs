using Cantrip.Client.Presentation;
using Cantrip.Protocol;
using Microsoft.UI.Text;

namespace Cantrip.Client.Web;

/// <summary>
/// The dock: what the server is asking, and whatever answers do not belong to a card, over the shared
/// <see cref="PromptSelection"/>; each <c>Fill</c> is checked against its screen in the prompt gallery.
/// </summary>
public sealed partial class MainPage
{
    private static KeyBinding PassKey => PlayerSettings.KeyFor(Hotkey.Pass);
    private static KeyBinding CancelKey => PlayerSettings.KeyFor(Hotkey.Cancel);
    private static KeyBinding AttackAllKey => PlayerSettings.KeyFor(Hotkey.AttackAll);

    private static string Suffix(KeyBinding key) => Hotkeys.Suffix(key);

    /// <summary>Where <see cref="AddButton"/> puts a button: the dock, or the choice window's footer while it carries the prompt.</summary>
    private FlowPanel _buttons = null!;
    private FlowPanel _dockButtons = null!;
    private TextBlock _errorLabel = null!;

    /// <summary>What the Pass, Cancel and Attack-all keys press on the dock as it stands; null where the dock has no such button.</summary>
    private Action? _pass;
    private Action? _cancel;
    private Action? _attackAll;

    /// <summary>
    /// Says what the server is asking and offers whatever does not belong to a card.
    /// </summary>
    private void FillPrompt(GameClient client)
    {
        ClearChoices();
        _dockButtons.Children.Clear();
        _buttons = _dockButtons;
        _pass = null;
        _attackAll = null;
        _cancel = null;

        if (!client.IsConnected)
        {
            Prompt("Disconnected", "The connection to the server was lost.");
            return;
        }

        if (client.View is null && client.Errors.Count > 0)
        {
            Prompt("Could not join", client.Errors[^1].Detail);
            return;
        }

        if (client.IsOver)
        {
            Prompt(DescribeOutcome(client), "The game is over.");
            ShowErrors(client);
            return;
        }

        // A draw another player offered is answered before anything else is (RX-4).
        if (AskDraw(client))
        {
            return;
        }

        if (_targeting is { } announced)
        {
            int candidates = announced.TargetCandidates.Count;
            // A cost that depends on the target (Dragonfire Blade's equip) is said with each, as a menu row says it: a card
            // with one thing to do comes straight here, with no row to have read it on.
            string costs = CardActions.CostsByTarget(announced);
            Prompt(
                $"{(announced.TargetsOptional ? "Up to one target" : "Target")} for {client.Object(announced.Source)?.Name ?? "it"}",
                (costs.Length > 0 ? costs + "  ·  " : string.Empty)
                    + (candidates == 1 ? "1 possible target" : $"{candidates} possible targets")
                    + (announced.TargetsOptional ? ", or none" : string.Empty));

            // "Up to one target" (CR 601.2c): aiming at nothing is a choice the action offers, not a way out of it.
            if (announced.TargetsOptional)
            {
                AddButton(_pickedTargets.Count == 0 ? "No target" : "Done with 1 target", () =>
                {
                    LetGraveyardsGo(client);
                    SendTargets(client);
                }, primary: true);
            }

            // Nothing has been announced yet, so backing out costs nothing and needs no message.
            AddCancelButton(() =>
            {
                _targeting = null;
                LetGraveyardsGo(client);
                _needsRedraw = true;
            });

            ShowErrors(client);
            return;
        }

        // The permanent a cost taps or sacrifices, picked on the board the way a target is.
        if (_selection.CostChoice is [var costing, ..])
        {
            int candidates = _costTakes.Count;
            Prompt(
                $"{client.Object(costing.Source)?.Name ?? "An ability"}: choose a permanent to {CardActions.CostVerb(costing)}",
                $"{costing.CostText}  ·  {(candidates == 1 ? "1 possible choice" : $"{candidates} possible choices")}"
                    + "  ·  click it on the battlefield");

            AddCancelButton(() =>
            {
                _selection.CostChoice = null;
                _needsRedraw = true;
            });

            ShowErrors(client);
            return;
        }

        switch (client.Prompt?.Prompt)
        {
            case PriorityPromptMessage priority:
                Prompt(Moment(client), OnTopOfStack(client) ?? Available(client, priority));
                _pass = () => Send(client, new PassAnswer());
                AddButton(PassLabel(client), _pass, primary: true).Tooltip = "Pass priority" + Suffix(PassKey);
                AddResolveAllButton(client);
                break;

            case DiscardPromptMessage discard:
                Prompt(
                    discard.Count == 1 ? "Discard a card" : $"Discard {discard.Count} cards",
                    $"{_picked.Count} of {discard.Count} chosen");

                AddButton(
                    "Discard",
                    () => Send(client, new CardsChosenAnswer { Cards = [.. _picked.Select(h => new ViewHandle(h))] }),
                    enabled: _picked.Count == discard.Count,
                    primary: true);

                break;

            case PayCostsPromptMessage pay:
                FillPayment(client, pay);
                break;

            case SearchLibraryPromptMessage search:
                FillSearch(client, search);
                break;

            case ChooseCardsPromptMessage choose:
                FillChooseCards(client, choose);
                break;

            case ChooseTargetPromptMessage target:
                FillChooseTarget(client, target);
                break;

            case ChooseDividedTargetsPromptMessage divide:
                FillDivideTargets(client, divide);
                break;

            case RemoveCountersPromptMessage remove:
                FillRemoveCounters(client, remove);
                break;

            case UnlessPaysPromptMessage unless:
                FillUnlessPays(client, unless);
                break;

            case MayPromptMessage may:
                FillMay(client, may);
                break;

            case ChooseModesPromptMessage modes:
                FillChooseModes(client, modes);
                break;

            case OrderTriggersPromptMessage triggers:
                FillOrderTriggers(client, triggers);
                break;

            case ChooseReplacementPromptMessage replacement:
                FillChooseReplacement(client, replacement);
                break;

            case NameCardPromptMessage naming:
                FillNameCard(client, naming);
                break;

            case ChoosePlayerPromptMessage choosing:
                FillChoosePlayer(client, choosing);
                break;

            case ChooseStartingPlayerPromptMessage starting:
                FillChooseStartingPlayer(client, starting);
                break;

            case MulliganPromptMessage mulligan:
                FillMulligan(client, mulligan);
                break;

            case ChooseColorsPromptMessage colors:
                FillChooseColors(client, colors);
                break;

            case DistributeCardsPromptMessage distribute:
                FillDistribute(client, distribute);
                break;

            case DeclareAttackersPromptMessage attackers:
                FillAttackers(client, attackers);
                break;

            case DeclareBlockersPromptMessage blockers:
                FillBlockers(client, blockers);
                break;

            case AssignCombatDamagePromptMessage assign:
                FillAssignDamage(client, assign);
                break;

            // This seat's player is controlled by another (CR 723): that player makes its decisions this turn, and the page
            // says so rather than that it waits for itself.
            case null when client.Me is { ControlledBy: > 0 } me && me.ControlledBy != client.Seat:
                Prompt(Moment(client), $"{NameOf(client, me.ControlledBy)} controls you this turn and makes your decisions");
                break;

            case null:
                // Nobody holds priority while another player is answering a prompt of their own, so the wait is named
                // after the other player rather than after nobody.
                Prompt(
                    client.Seat == 0 ? "Joining" : client.View is { Turn: 0 } ? "The game is starting" : Moment(client),
                    client.Seat == 0
                        ? "Waiting for a seat"
                        : client.View?.PriorityPlayer is > 0 and var holder
                            ? $"Waiting for {NameOf(client, holder)}"
                            : "Waiting for the other player");

                break;

            case { } unknown:
                // A question this build has no dock for: the prompt knows the simplest legal answer to itself.
                Prompt(Spaced(unknown.GetType().Name), "This build has no screen for this question yet.");
                AddButton("Continue", () => Send(client, unknown.Default()), primary: true);
                break;
        }

        // A spell or ability still being announced can be given up at any of its questions.
        if (client.Prompt is { MayCancel: true })
        {
            AddCancelButton(() => Send(client, new CancelAnnouncementAnswer()));
        }

        ShowErrors(client);
    }

    /// <summary>A readable title for a prompt this build has no dock for: <c>ChooseModesPromptMessage</c> reads as "Choose modes".</summary>
    private static string Spaced(string typeName)
    {
        string bare = typeName.EndsWith("PromptMessage", StringComparison.Ordinal)
            ? typeName[..^"PromptMessage".Length]
            : typeName;

        var text = new System.Text.StringBuilder();

        foreach (char letter in bare)
        {
            if (char.IsUpper(letter) && text.Length > 0)
            {
                text.Append(' ').Append(char.ToLowerInvariant(letter));
            }
            else
            {
                text.Append(letter);
            }
        }

        return text.Length == 0 ? "A decision" : text.ToString();
    }

    /// <summary>What the Auto-mana key does while a cost is being paid, for the dock line to end with.</summary>
    private string AutoManaNote()
    {
        if (!_actions.Offered.Any(option => AutoMana.IsManaAbility(option) && Ways(option) > 1))
        {
            return string.Empty;
        }

        KeyBinding key = PlayerSettings.KeyFor(Hotkey.AutoMana);
        return PlayerSettings.AutoManaByDefault
            ? $"  ·  auto-mana is picking, hold {key} to choose"
            : $"  ·  hold {key} to have the colour picked";

        int Ways(ActionOptionMessage option) =>
            option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0
                ? Math.Max(option.ColorChoices.Count, 2)
                : _actions.For(option.Source).Count;
    }

    // --- Paying ---

    private void FillPayment(GameClient client, PayCostsPromptMessage prompt)
    {
        string name = client.Object(prompt.Spell)?.Name ?? "the spell";
        bool owing = AutoPlay.Owing(prompt);

        // A {C} still owed (CR 107.4c) is paid only with colorless mana.
        string colorless = prompt.RemainingColorless > 0 ? "  ·  only colorless mana pays {C}" : string.Empty;

        if (prompt.AssistStep.Length > 0 && FillAssist(client, prompt, name))
        {
            return;
        }

        if (prompt.AnnouncesXOrKicks)
        {
            FillAnnouncement(client, prompt, name);
            return;
        }

        if (!prompt.HasX)
        {
            // The pool covers the cost in one way, and something other than mana could pay some of it instead
            // (CR 601.2h): the player confirms the pool, or taps or exiles to keep some of it.
            bool covered = !owing && _actions.Confirm is not null;

            string how = covered
                ? "Okay pays it from your pool"
                : _actions.Assignable.Count > 0
                ? _actions.Any
                    ? "click mana in your pool, or tap a land"
                    : "click mana in your pool"
                : _actions.Any
                    ? "tap a land"
                    : "nothing untapped can pay it";

            // Waterbend (CR 701.67a) and improvise (CR 702.126a).
            List<ActionOptionMessage> tapping = prompt.Options.FindAll(option => option.Kind == ActionKindView.TapToPay);
            if (tapping.Count > 0)
            {
                bool artifactsOnly = tapping.TrueForAll(option =>
                    client.Object(option.Source)?.CardTypes.Contains(CardTypeView.Artifact) ?? false);

                // Convoke (CR 702.51a) describes each tap, since a creature may pay one of its colors as well as {1}.
                how = tapping.Exists(option => option.Description.Length > 0)
                    ? $"{how}  ·  or tap a creature to help pay"
                    : $"{how}  ·  or tap an {(artifactsOnly ? "artifact" : "artifact or creature")} for {{1}}";
            }

            // Delve (CR 702.66a).
            if (prompt.Options.Exists(option => option.Kind == ActionKindView.ExileToPay))
            {
                how = $"{how}  ·  or exile a card from your graveyard for {{1}}";
            }

            // A Phyrexian mana symbol (CR 107.4f).
            if (_actions.PayLife.Count > 0)
            {
                how = $"{how}  ·  or pay life for {{{CardActions.Symbol(_actions.PayLife[0].Produces[0])}/P}}";
            }

            // Web-slinging (CR 702.188a), Urban Retreat: the creature returned must be tapped once the mana is paid, so while none of the
            // player's creatures is, the caption says how one becomes tapped; a payment that leaves none is taken back.
            if (_tappedAfterMana && !client.BattlefieldOf(client.ActingSeat).Any(card => card.Tapped && card.CardTypes.Contains(CardTypeView.Creature)))
            {
                how = $"{how}  ·  tap a creature for mana: it will be the one returned";
            }

            // "You can't spend mana to cast this spell" (CR 601.2h, 106.4): lands and pool mana are no help, whatever the
            // board offers, so only what is not mana is named.
            if (prompt.SpendsNoMana)
            {
                List<string> ways = [];
                if (prompt.Options.Exists(option => option.Kind == ActionKindView.TapToPay))
                {
                    ways.Add("tap a creature to help pay");
                }

                if (prompt.Options.Exists(option => option.Kind == ActionKindView.ExileToPay))
                {
                    ways.Add("exile a card from your graveyard for {1}");
                }

                how = ways.Count > 0
                    ? $"no mana can pay it  ·  {string.Join("  ·  or ", ways)}"
                    : "no mana can pay it, and nothing else left can";
                colorless = string.Empty;
            }

            // A cost to attack (CR 508.1h-j): what it pays for is the attack, not the permanent that makes attacking cost
            // something; giving it up takes the declaration back.
            if (prompt.Attackers.Count > 0)
            {
                string attackers = prompt.Attackers.Count == 1
                    ? client.Object(prompt.Attackers[0])?.Name ?? "1 creature"
                    : $"{prompt.Attackers.Count} creatures";
                // The life chosen for its Phyrexian symbols (CR 107.4f: Norn's Annex's {W/P}), paid with the mana.
                string life = prompt.Life > 0 ? $" and {prompt.Life} life" : string.Empty;
                Prompt(
                    $"Pay {prompt.Total}{life} to attack with {attackers}",
                    covered
                        ? $"{name} asks it  ·  your pool covers it  ·  {how}"
                        : $"{name} asks it  ·  {prompt.Remaining} still to pay  ·  {how}  ·  Cancel declares attackers again");
            }
            else
            {
                // Life the total cost asks besides its mana (CR 119.4, 601.2h: Terror of the Peaks' additional 3 life), paid
                // with it once the mana is.
                string life = prompt.Life > 0 ? $"  ·  and {prompt.Life} life, paid with it" : string.Empty;

                // Assist (CR 702.132a): the caster pays what the player helping did not, and no mana can be made now.
                string helped = prompt.AssistStep != AssistSteps.CasterPays
                    ? string.Empty
                    : prompt.Assisted > 0
                        ? $"  ·  {prompt.Assisted} paid by the player helping"
                        : "  ·  the player helping paid none";
                Prompt(
                    _actions.Assignable.Count > 0 ? $"Choose mana for {name}" : $"Pay for {name}",
                    covered
                        ? $"your pool covers {prompt.Total}{life}{helped}  ·  {how}"
                        : $"{prompt.Remaining} of {prompt.Total} still to pay{life}{helped}  ·  {how}{colorless}{(prompt.SpendsNoMana ? string.Empty : AutoManaNote())}");
            }

            if (covered && _actions.Confirm is { } pay)
            {
                _pass = () => Send(client, AnswerFor(pay));
                AddButton("Okay", () => Send(client, AnswerFor(pay)), primary: true).Tooltip =
                    $"Pay {prompt.Total} with the mana in your pool" + Suffix(PassKey);
            }

            foreach (ActionOptionMessage life in _actions.PayLife)
            {
                AddButton($"Pay {life.LifeCost} life", () => Send(client, AnswerFor(life))).Tooltip =
                    $"Pay {life.LifeCost} life instead of {{{CardActions.Symbol(life.Produces[0])}}}";
            }

            if (_actions.Cancel is { } cancel)
            {
                AddCancelButton(() => Send(client, AnswerFor(cancel)));
            }

            return;
        }

        // Kicker (CR 702.33) and buyback (CR 702.27): the count being chosen is how many times that cost is paid.
        bool kicker = prompt.Kicker.Length > 0;
        string raise = _actions.IncreaseX is null
            ? string.Empty
            : !kicker
            ? "  ·  tap a land or click mana in your pool to raise X"
            : prompt.Squad
                ? $"  ·  {Announcement.SquadRaiseHint(prompt)}"
            : prompt.Buyback
                ? $"  ·  tap a land or click mana in your pool to buy it back for {prompt.Kicker}"
                : prompt.X == 0
                    ? $"  ·  tap a land or click mana in your pool to kick it for {prompt.Kicker}"
                    : $"  ·  tap a land or click mana in your pool to kick it again for {prompt.Kicker}";

        Prompt(
            Announcement.Title(prompt, name),
            owing
                ? $"{prompt.Remaining} of {prompt.Total} still to pay  ·  tap a land or click mana in your pool{colorless}"
                : $"{prompt.Total} paid{raise}");

        if (_actions.Confirm is { } confirm)
        {
            _pass = () => Send(client, AnswerFor(confirm));
            AddButton("Okay", () => Send(client, AnswerFor(confirm)), primary: true).Tooltip =
                (kicker ? $"Cast {Paid(prompt, prompt.X, prompt.KickerPaid)}" : $"Cast with X = {prompt.X}") + Suffix(PassKey);
        }
        else
        {
            AddButton("Okay", () => { }, enabled: false).Tooltip = "Pay what is still owed first.";
        }

        AddXStepper(
            prompt,
            prompt.X,
            prompt.KickerPaid,
            _actions.DecreaseX is { } down ? () => { Send(client, AnswerFor(down)); RedrawNow(); } : null,
            _actions.IncreaseX is { } up ? () => { Send(client, AnswerFor(up)); RedrawNow(); } : null);

        if (_actions.Cancel is { } giveUp)
        {
            AddCancelButton(() => Send(client, AnswerFor(giveUp)));
        }
    }

    /// <summary>
    /// The steps of paying for a spell with assist (CR 702.132a) that are not an ordinary payment: each player's window for mana
    /// abilities, closed with Done - the helper's with no Cancel, since the spell is not theirs - and the helper's announcement
    /// of how much of the generic mana they pay. The helper's payment and the caster's are drawn as any payment is, with a line
    /// saying whose spell it is or what the other player paid. False for a step this does not draw.
    /// </summary>
    private bool FillAssist(GameClient client, PayCostsPromptMessage prompt, string name)
    {
        string caster = client.Object(prompt.Spell) is { } spell && spell.Controller != client.ActingSeat ? NameOf(client, spell.Controller) : "Its caster";
        switch (prompt.AssistStep)
        {
            // A window for mana abilities (CR 601.2g): what the question's Remaining says is the whole cost, not what is left.
            case AssistSteps.HelperActivates or AssistSteps.CasterActivates:
            {
                bool helper = prompt.AssistStep == AssistSteps.HelperActivates;
                Prompt(
                    helper ? $"Help pay for {name}: make mana" : $"Make mana for {name}",
                    helper
                        ? $"{caster} chose you to help pay for {name}  ·  tap lands for mana now, then choose how much of its {prompt.Total} generic mana you pay"
                        : $"{name} costs {prompt.Total}  ·  the last chance to make mana: once you are done, you pay from your pool");
                if (_actions.Confirm is { } done)
                {
                    _pass = () => Send(client, AnswerFor(done));
                    AddButton("Done", () => Send(client, AnswerFor(done)), primary: true).Tooltip =
                        (helper ? "Done making mana" : "Done making mana: pay from your pool") + Suffix(PassKey);
                }

                if (_actions.Cancel is { } cancel)
                {
                    AddCancelButton(() => Send(client, AnswerFor(cancel)));
                }

                return true;
            }

            // How much of the generic mana the helper pays (CR 702.132a): at most the generic mana and what their pool holds.
            case AssistSteps.HelperAnnounces:
            {
                int generic = prompt.RemainingGeneric;
                int x = Announced(prompt);
                Prompt(
                    $"How much of the {{{generic}}} generic mana of {name} do you pay?",
                    $"{caster} pays the rest  ·  your pool can pay up to {prompt.MostAnnounced}");
                Action announce = () => Send(client, new AnnounceAnswer { Value = x });
                if (_actions.Confirm is not null)
                {
                    _pass = announce;
                    AddButton("Okay", announce, primary: true).Tooltip = $"Pay {x} of it from your pool" + Suffix(PassKey);
                }

                Action? lower = x > prompt.LeastAnnounced ? () => StepAnnounced(prompt, x - 1) : null;
                Action? raise = x < prompt.MostAnnounced ? () => StepAnnounced(prompt, x + 1) : null;
                _buttons.Children.Add(Stepper(
                    8,
                    AssistStep("−", "Pay one less", lower),
                    new SymbolText(13, Theme.Fg, semibold: true) { Text = $"Pay {x}" },
                    AssistStep("+", "Pay one more", raise)));
                return true;
            }

            // The helper pays what they announced from their pool, as any payment from a pool is made, with no giving up.
            case AssistSteps.HelperPays:
                Prompt($"Help pay for {name}", $"{prompt.Remaining} still to pay from your pool  ·  click mana in your pool");
                if (_actions.Confirm is { } covered)
                {
                    _pass = () => Send(client, AnswerFor(covered));
                    AddButton("Okay", () => Send(client, AnswerFor(covered)), primary: true).Tooltip = "Pay it with the mana in your pool" + Suffix(PassKey);
                }

                return true;

            default:
                return false;
        }

        static DockButton AssistStep(string text, string tooltip, Action? step)
        {
            var button = new DockButton(text, primary: false, enabled: step is not null) { Tooltip = tooltip };
            if (step is not null)
            {
                button.Clicked += step;
            }

            return button;
        }
    }

    /// <summary>
    /// The announcement (CR 601.2b): X, or how many times to kick, chosen before the targets and before anything is paid. The
    /// value is stepped on the page, with nothing sent, and announced at once as one <see cref="AnnounceAnswer"/>; for an
    /// {X} paid in mana, a mana source clicked is planned for it (<see cref="TogglePlanned"/>) and raises X to what the mana
    /// reaches.
    /// </summary>
    private void FillAnnouncement(GameClient client, PayCostsPromptMessage prompt, string name)
    {
        int x = Announced(prompt);
        bool kickerPaid = Announcement.KickerPaid(prompt, x);
        string total = Announcement.TotalAt(prompt, x);
        Action? announce = _actions.Confirm is null ? null : () => Send(client, new AnnounceAnswer { Value = x });
        Action? lower = x > prompt.LeastAnnounced ? () => StepAnnounced(prompt, x - 1) : null;
        Action? raise = x < prompt.MostAnnounced ? () => StepAnnounced(prompt, x + 1) : null;

        // A "−X" loyalty cost (CR 606.4, 107.3): X loyalty counters come off the permanent, up to its loyalty; no mana.
        if (prompt.XRemovesLoyalty)
        {
            Prompt(
                $"Choose X for {name}",
                $"−X removes X loyalty counters  ·  {name} has {prompt.MostAffordable}  ·  targets come next");

            if (announce is not null)
            {
                _pass = announce;
                AddButton("Okay", announce, primary: true).Tooltip =
                    $"Activate with X = {x}, removing {x} loyalty" + Suffix(PassKey);
            }

            AddXStepper(prompt, x, kickerPaid, lower, raise);

            if (_actions.Cancel is { } back)
            {
                AddCancelButton(() => Send(client, AnswerFor(back)));
            }

            return;
        }

        // "You may sacrifice any number of nonland permanents. This spell costs {1} less to cast for each permanent sacrificed
        // this way" (CR 601.2b, 601.2f): how many is announced now, and which ones are chosen once the mana is paid.
        if (prompt.SacrificeReduces > 0)
        {
            Prompt(
                $"Sacrifice how many for {name}?",
                $"{x} of up to {prompt.MostAffordable}  ·  each takes {{{prompt.SacrificeReduces}}} off  ·  {total} in all  ·  which ones come after the mana");

            if (announce is not null)
            {
                _pass = announce;
                AddButton("Okay", announce, primary: true).Tooltip =
                    $"Cast it sacrificing {x}" + Suffix(PassKey);
            }

            AddXStepper(prompt, x, kickerPaid, lower, raise);

            if (_actions.Cancel is { } back)
            {
                AddCancelButton(() => Send(client, AnswerFor(back)));
            }

            return;
        }

        // An X that is not mana's (casualty X, CR 702.153a): the cost says what it governs, and mana is no part of reaching it.
        if (prompt.XGoverns.Length > 0)
        {
            Prompt(
                $"Choose X for {name}",
                $"X is {prompt.XGoverns}  ·  {total} in all  ·  targets and paying come next");

            if (announce is not null)
            {
                _pass = announce;
                AddButton("Okay", announce, primary: true).Tooltip = $"Announce X = {x}" + Suffix(PassKey);
            }

            AddXStepper(prompt, x, kickerPaid, lower, raise);

            if (_actions.Cancel is { } back)
            {
                AddCancelButton(() => Send(client, AnswerFor(back)));
            }

            return;
        }

        bool kicker = prompt.Kicker.Length > 0;
        string reach = prompt.MostAffordable < 0
            ? "not enough mana for it"
            : prompt.XPaidInLife
                ? prompt.MostAffordable > 0
                    ? $"X is paid in life: up to {prompt.MostAffordable} leaves you alive"
                    : "X is paid in life, and any at all would leave you with none"
            : prompt.VariableKicker
                ? prompt.MostAffordable >= 0 ? $"enough mana for X up to {prompt.MostAffordable}" : "not enough mana to kick it"
            : kicker
                ? prompt.Squad
                    ? Announcement.SquadReach(prompt)
                : prompt.Buyback
                    ? prompt.MostAffordable > 0 ? "you have the mana to buy it back" : "not enough mana to buy it back"
                    : prompt.MostAffordable > 0
                        ? $"enough mana to kick it {Times(prompt.MostAffordable)}"
                        : "not enough mana to kick it"
                : $"enough mana for X up to {prompt.MostAffordable}";

        // Mana planned for X is made once the targets are chosen (CR 601.2g): until then the caption says how much there is.
        string planning = !PlansMana(prompt)
            ? string.Empty
            : _planned.Count > 0
                ? $"  ·  {ManaForX(client)} mana for it, tapped as it is paid"
                : "  ·  tap lands to raise X";

        Prompt(
            Announcement.Title(prompt, name),
            prompt.XPaidInLife
                ? $"{total} and {x} life  ·  {reach}  ·  targets and paying come next"
                : $"{total} in all  ·  {reach}{planning}  ·  targets and paying come next");

        if (announce is not null)
        {
            _pass = announce;
            AddButton("Okay", announce, primary: true).Tooltip =
                (kicker ? $"Cast {Paid(prompt, x, kickerPaid)}" : $"Announce X = {x}") + Suffix(PassKey);
        }

        AddXStepper(prompt, x, kickerPaid, lower, raise);

        if (_actions.Cancel is { } giveUp)
        {
            AddCancelButton(() => Send(client, AnswerFor(giveUp)));
        }

        static string Times(int times) => times switch
        {
            1 => "once",
            _ => $"{times} times",
        };
    }

    /// <summary>The value being announced: the page's own once it has been stepped, else the one the question was asked at.</summary>
    private int Announced(PayCostsPromptMessage prompt) => Announcement.Clamp(prompt, _selection.Announced ?? prompt.X);

    /// <summary>Steps the value being announced on the page, sending nothing: the dock is drawn again at once with it.</summary>
    private void StepAnnounced(PayCostsPromptMessage prompt, int value)
    {
        _selection.Announced = Announcement.Clamp(prompt, value);
        _needsRedraw = true;
        RedrawNow();
    }

    private static string Paid(PayCostsPromptMessage prompt, int x, bool kickerPaid) => prompt.Squad
        ? Announcement.SquadPaid(x)
        : prompt.VariableKicker
        ? kickerPaid ? $"kicked with X = {x}" : "not kicked"
        : prompt.Buyback
        ? x == 0 ? "without buyback" : "with buyback"
        : x switch
        {
            0 => "not kicked",
            1 => "kicked",
            var times => $"kicked {times} times",
        };

    /// <summary>
    /// The step down, the value, and the step up, as one group the dock's flow cannot wrap apart; a step with nothing to do
    /// is drawn disabled.
    /// </summary>
    private void AddXStepper(PayCostsPromptMessage prompt, int x, bool kickerPaid, Action? lower, Action? raise)
    {
        bool kicker = prompt.Kicker.Length > 0;
        string down = !kicker ? "Lower X by one" : prompt.Squad ? "Pay squad one time fewer" : prompt.VariableKicker
            ? "Lower X, or decline the kicker" : prompt.Buyback ? "Cast it without buying it back" : "Kick it one time fewer";
        string up = !kicker
            ? "Raise X by one"
            : prompt.VariableKicker ? kickerPaid ? "Raise kicker X by one" : $"Pay kicker {prompt.Kicker}"
            : prompt.Squad ? Announcement.SquadRaiseHint(prompt) : prompt.Buyback ? $"Buy it back for {prompt.Kicker}" : $"Kick it once more for {prompt.Kicker}";

        _buttons.Children.Add(Stepper(
            8,
            StepButton("−", down, lower),
            new SymbolText(13, Theme.Fg, semibold: true) { Text = kicker ? Paid(prompt, x, kickerPaid) : $"X = {x}" },
            StepButton("+", up, raise)));

        static DockButton StepButton(string text, string tooltip, Action? step)
        {
            var button = new DockButton(text, primary: false, enabled: step is not null) { Tooltip = tooltip };
            if (step is not null)
            {
                button.Clicked += step;
            }

            return button;
        }
    }

    /// <summary>A step down, what it steps, and a step up, in one row.</summary>
    private static StackPanel Stepper(double spacing, FrameworkElement lower, FrameworkElement label, FrameworkElement raise)
    {
        label.VerticalAlignment = VerticalAlignment.Center;
        return new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = spacing,
            HorizontalAlignment = HorizontalAlignment.Left,
            Children = { lower, label, raise },
        };
    }

    // --- Targets ---

    /// <summary>
    /// The slot's own words with the opponent it is for named, ending in the separator the dock puts before its count; empty when
    /// the card has nothing to say of its slot.
    /// </summary>
    private static string SlotLabel(GameClient client, ChooseTargetPromptMessage prompt)
    {
        if (prompt.Text.Length == 0)
        {
            return string.Empty;
        }

        string opponent = prompt.ForPlayer == 0 ? "an opponent"
            : client.View?.Players.FirstOrDefault(player => player.Seat == prompt.ForPlayer)?.Name ?? $"seat {prompt.ForPlayer}";
        return $"{prompt.Text.Replace("{player}", opponent, StringComparison.Ordinal)}  ·  ";
    }

    /// <summary>Choosing a triggered ability's target as it goes on the stack (CR 603.3d).</summary>
    private void FillChooseTarget(GameClient client, ChooseTargetPromptMessage prompt)
    {
        if (!_peekedForTargets)
        {
            _peekedForTargets = true;
            PeekAtTargets(client, prompt.Candidates);
        }

        string source = client.Object(prompt.Source)?.Name ?? client.Object(prompt.Ability)?.Name ?? "An ability";
        int count = prompt.Candidates.Count;

        // Choosing is not targeting (CR 608.2d).
        string noun = prompt.Targeted ? "target" : "choice";
        string nouns = prompt.Targeted ? "targets" : "choices";

        // What this slot is for, where the card says (Jeskai Revelation's two slots, The Theorist's one for each opponent).
        string slot = SlotLabel(client, prompt);

        // "With total mana value X or less" (CR 601.2c): the running total of what is picked, against the limit; any other
        // rule of the targets together in its own words ("with different mana values, with mana value 3 or less").
        if (prompt.MaximumTotalManaValue is { } most)
        {
            slot = $"{slot}total mana value {prompt.TotalManaValueOf(_pickedTargets)}, at most {most}  ·  ";
        }
        else if (prompt.Words.Count > 0)
        {
            slot = $"{slot}{string.Join(", ", prompt.Words)}  ·  ";
        }

        Prompt(
            prompt.AnyNumber ? $"Any number of {nouns} for {source}"
                : prompt.Count == 1 && prompt.Optional ? $"Up to one {noun} for {source}"
                : prompt.Count == 1 ? $"{(prompt.Targeted ? "Target" : "Choice")} for {source}"
                : prompt.Optional ? $"Up to {prompt.Count} {nouns} for {source}"
                : prompt.Minimum > 0 ? $"{prompt.Minimum} to {prompt.Count} {nouns} for {source}"
                : $"{prompt.Count} {nouns} for {source}",
            prompt.Count == 1 && !prompt.AnyNumber
                ? $"{slot}{(count == 1 ? $"1 possible {noun}" : $"{count} possible {nouns}")}"
                : prompt.AnyNumber
                ? $"{slot}{_pickedTargets.Count} of {count} chosen"
                : $"{slot}{_pickedTargets.Count} of {prompt.Count} chosen, from {count}");

        if (prompt.Optional)
        {
            AddButton(
                _pickedTargets.Count switch
                {
                    0 => prompt.AnyNumber ? "Choose none" : prompt.Count == 1 ? $"No {noun}" : $"No {nouns}",
                    1 => $"Done with 1 {noun}",
                    var picked => $"Done with {picked} {nouns}",
                },
                () => SendTargets(client),
                primary: true);
        }
        else if (prompt.Minimum > 0)
        {
            // A printed range, "one or two target creatures" (CR 601.2c): done once the minimum is reached.
            AddButton(
                _pickedTargets.Count == 1 ? $"Done with 1 {noun}" : $"Done with {_pickedTargets.Count} {nouns}",
                () => SendTargets(client),
                enabled: _pickedTargets.Count >= prompt.Minimum,
                primary: true);
        }

        // A non-targeted choice may name revealed library cards (Fact or Fiction). They have no row on the board;
        // show only the offered handles, never the rest of the library.
        List<ObjectView> libraryCards = [.. prompt.Candidates.Where(candidate => !candidate.IsPlayer)
            .Select(candidate => client.Object(candidate.Object)).OfType<ObjectView>()
            .Where(card => card.Zone == ZoneKindView.Library)];
        if (libraryCards.Count > 0)
        {
            OpenChoices($"Revealed library cards  ·  {libraryCards.Count} cards", closable: false, libraryCards.Count);
            foreach (ObjectView card in libraryCards)
            {
                AddChoice(card, LookOf(client, card));
            }
        }
    }

    /// <summary>
    /// Choosing a divided slot's targets and how its total splits among them together (CR 601.2d). The candidates on the
    /// battlefield wear their shares, with a step down and up, the way blockers wear combat damage, so whose creature
    /// gets what is read off the board; the dock says what has been chosen, and whose each one is.
    /// </summary>
    private void FillDivideTargets(GameClient client, ChooseDividedTargetsPromptMessage prompt)
    {
        if (!_peekedForTargets)
        {
            _peekedForTargets = true;
            PeekAtTargets(client, prompt.Candidates);
        }

        string source = client.Object(prompt.Source)?.Name ?? client.Object(prompt.Ability)?.Name ?? "An ability";
        int assigned = _dividedShares.Values.Sum();
        int targeted = _dividedShares.Count(pair => pair.Value > 0);

        Prompt(
            $"Divide {prompt.Total} for {source}",
            $"{prompt.Text}  ·  {assigned} of {prompt.Total} assigned, among {targeted} target{(targeted == 1 ? string.Empty : "s")}{(prompt.MaxTargets > 0 ? $" (up to {prompt.MaxTargets})" : string.Empty)}{(prompt.Optional ? "  ·  or no target at all" : string.Empty)}  ·  click a card to add one, or use its − and +");

        bool none = prompt.Optional && assigned == 0;

        AddButton(
            none ? "Divide nothing" : "Divide",
            () => Send(client, new TargetsDividedAnswer
            {
                Shares =
                [
                    .. _dividedShares
                        .Where(pair => pair.Value > 0)
                        .Select(pair => new TargetShareMessage
                        {
                            Target = new TargetMessage { Object = new ViewHandle(pair.Key) },
                            Amount = pair.Value,
                        }),
                ],
            }),
            enabled: none || assigned == prompt.Total && (prompt.MaxTargets == 0 || targeted <= prompt.MaxTargets),
            primary: true).Tooltip = (none ? "Choose no target" : "Lock in the division") + Suffix(PassKey);

        if (assigned > 0)
        {
            AddButton("Clear", () =>
            {
                _dividedShares.Clear();
                _needsRedraw = true;
            });
        }

        foreach (TargetMessage candidate in prompt.Candidates)
        {
            if (candidate.IsPlayer)
            {
                continue;
            }

            int handle = candidate.Object.Value;
            int share = _dividedShares.GetValueOrDefault(handle);
            ObjectView? card = client.Object(candidate.Object);
            string name = card is null ? "a creature" : $"{card.Name} ({(card.Controller == client.Seat ? "yours" : $"{NameOf(client, card.Controller)}'s")})";

            // On the battlefield the card itself carries the steps, and the dock only names what has a share.
            if (card?.Zone == ZoneKindView.Battlefield)
            {
                if (share > 0)
                {
                    _buttons.Children.Add(new SymbolText(13, Theme.Fg) { Text = $"{share}  {name}" });
                }

                continue;
            }

            bool canAddTarget = share > 0 || prompt.MaxTargets == 0 || targeted < prompt.MaxTargets;

            _buttons.Children.Add(Stepper(
                6,
                Step("−", share > 0, () => _dividedShares[handle] = Math.Max(0, share - 1)),
                new SymbolText(13, Theme.Fg) { Text = $"{share}  {name}" },
                Step("+", assigned < prompt.Total && canAddTarget, () => _dividedShares[handle] = share + 1)));
        }
    }

    /// <summary>
    /// Removing counters from among permanents (CR 118.3, 122.1), as a cost ("by removing three counters from among creatures you
    /// control") or as an effect resolves (Dyadrine's "a +1/+1 counter from each of two creatures you control"). Each kind of
    /// counter on each permanent the question may take from is a button that takes one more of them, saying how many of how many
    /// are taken; a click on a permanent takes one of its own, the first kind with some left. The answer is sent once the total
    /// is reached. When every pool holds a single counter on a different permanent, the subtitle says one counter comes from each;
    /// when every pool is of one kind (a cost that names it, "two +1/+1 counters"), it names the kind.
    /// An effect's "remove up to three counters from target creature" (<see cref="RemoveCountersPromptMessage.UpTo"/>: Heartless
    /// Act, CR 608.2d) is captioned "remove up to 3 counters", and its button removes however many are chosen, none included,
    /// saying how many: "Remove 2 counters", "Remove no counters".
    /// </summary>
    private void FillRemoveCounters(GameClient client, RemoveCountersPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "A spell";
        int taken = _selection.CounterRemovals.Values.Sum();
        string counters = prompt.Total == 1 ? "a counter" : $"{prompt.Total} counters";

        // One counter on each of different permanents (Dyadrine): a counter from each of that many permanents, not any kinds.
        bool oneEach = prompt.Pools.All(pool => pool.Count == 1) && prompt.Pools.Select(pool => pool.Object).Distinct().Count() == prompt.Pools.Count;
        // A cost of one kind ("two +1/+1 counters", Hopeful Initiate) offers only that kind's pools, and is captioned with it.
        List<string> kinds = [.. prompt.Pools.Select(pool => pool.Counter).Distinct()];
        string among = kinds.Count == 1 ? $"{kinds[0]} counters, from among the permanents shown" : "Any kinds, from among the permanents shown";
        string which = oneEach ? $"One counter from each of {prompt.Total} different permanents" : among;

        if (prompt.UpTo)
        {
            which = "Any kinds, as many as you choose, or none";
        }

        Prompt(
            prompt.UpTo ? $"{source}: remove up to {(prompt.Total == 1 ? "one counter" : counters)}" : $"{source}: remove {counters}",
            $"{which}  ·  {taken} of {(prompt.UpTo ? "up to " : string.Empty)}{prompt.Total} chosen  ·  click a permanent to take one of its counters, or a button below");

        string picked = taken switch
        {
            0 => "no counters",
            1 => "1 counter",
            _ => $"{taken} counters",
        };
        AddButton(
            prompt.UpTo ? $"Remove {picked}" : $"Remove {counters}",
            () => Send(client, new CountersRemovedAnswer
            {
                Removals =
                [
                    .. prompt.Pools
                        .Select(pool => (pool, n: _selection.CounterRemovals.GetValueOrDefault((pool.Object.Value, pool.Counter))))
                        .Where(chosen => chosen.n > 0)
                        .Select(chosen => new CounterPoolMessage { Object = chosen.pool.Object, Counter = chosen.pool.Counter, Count = chosen.n }),
                ],
            }),
            enabled: prompt.UpTo || taken == prompt.Total,
            primary: true).Tooltip = (prompt.UpTo && taken == 0 ? "Remove none of the counters" : $"Remove the {(prompt.UpTo ? picked : counters)} chosen") + Suffix(PassKey);

        if (taken > 0)
        {
            AddButton("Clear", () =>
            {
                _selection.CounterRemovals.Clear();
                _needsRedraw = true;
            });
        }

        foreach (CounterPoolMessage pool in prompt.Pools)
        {
            int n = _selection.CounterRemovals.GetValueOrDefault((pool.Object.Value, pool.Counter));
            ObjectView? card = client.Object(pool.Object);
            string name = card?.Name ?? "a permanent";

            // Two permanents of one name are told apart by their order on the battlefield: "Walking Ballista #2".
            List<ViewHandle> named = [.. prompt.Pools.Select(p => p.Object).Distinct().Where(h => client.Object(h)?.Name == card?.Name)];
            if (named.Count > 1)
            {
                name += $" #{named.IndexOf(pool.Object) + 1}";
            }
            AddButton(
                $"Take a {pool.Counter} counter from {name} ({n} of {pool.Count})",
                () =>
                {
                    _selection.CounterRemovals[(pool.Object.Value, pool.Counter)] = n + 1;
                    _needsRedraw = true;
                },
                enabled: n < pool.Count && taken < prompt.Total);
        }
    }

    /// <summary>
    /// A click on a permanent while counters are being removed from among permanents: one more of its counters, of the first
    /// kind it still has some of, while the total is not reached.
    /// </summary>
    private void TakeCounterFrom(RemoveCountersPromptMessage prompt, ViewHandle permanent)
    {
        if (_selection.CounterRemovals.Values.Sum() >= prompt.Total)
        {
            return;
        }

        foreach (CounterPoolMessage pool in prompt.Pools.Where(pool => pool.Object == permanent))
        {
            int n = _selection.CounterRemovals.GetValueOrDefault((pool.Object.Value, pool.Counter));
            if (n < pool.Count)
            {
                _selection.CounterRemovals[(pool.Object.Value, pool.Counter)] = n + 1;
                _needsRedraw = true;
                return;
            }
        }
    }

    /// <summary>A − or + beside a share: changes it and redraws.</summary>
    private DockButton Step(string text, bool enabled, Action change)
    {
        var button = new DockButton(text, primary: false, enabled: enabled);
        button.Clicked += () =>
        {
            change();
            _needsRedraw = true;
            RedrawNow();
        };

        return button;
    }

    // --- Modes, players, colours, names, replacements ---

    /// <summary>Past this many options the dock does not lay out a button for each; it finds them by typing instead.</summary>
    private const int LongOptionList = 12;

    /// <summary>How many of a long list's options, the best matches for what is typed, the dock shows at once.</summary>
    private const int OptionsShown = 8;

    /// <summary>
    /// Choosing a modal spell's modes (CR 700.2), or one of a card's named options (CR 614.12a): one button per mode, or
    /// for a long list - every creature type (CR 205.3m) - a box to type in and buttons for the options it matches.
    /// </summary>
    /// <summary>How many modes a range takes, in the words the card would use: "choose one or both", "choose up to two".</summary>
    private static string RangeTitle(ChooseModesPromptMessage prompt) =>
        prompt.Count == 0 ? $"choose up to {Spelled(prompt.Most)}"
        : prompt.Count == 1 && prompt.Most == 2 ? "choose one or both"
        : $"choose {Spelled(prompt.Count)} to {Spelled(prompt.Most)}";

    private void FillChooseModes(GameClient client, ChooseModesPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "A spell";
        // Every one of two modes is "both", in the card's words: "If this spell's additional cost was paid, choose both instead".
        string count = prompt.Count == 2 && prompt.Modes.Count == 2 && !prompt.MayRepeat ? "both" : Spelled(prompt.Count);
        bool range = prompt.Most > prompt.Count;
        bool oneClick = prompt.Count == 1 && !range;
        bool searched = prompt.Modes.Count > LongOptionList;

        Prompt(
            prompt.Heading.Length > 0 ? prompt.Heading
                : prompt.AdditionalCost ? $"{source}: choose an additional cost to pay"
                : range ? $"{source}: {RangeTitle(prompt)}"
                : $"{source}: choose {count}",
            (prompt.Heading.Length > 0
                ? prompt.Detail.Length > 0 ? prompt.Detail
                : prompt.Source.IsNone ? "Its room ability triggers as you move in" : "Chosen, not targeted"
                : prompt.AdditionalCost
                ? "Paid along with its mana cost"
                : oneClick
                ? prompt.Detail.Length > 0 ? prompt.Detail : "Each mode's target is chosen next"
                : range
                ? prompt.Detail.Length > 0 ? $"{_pickedModes.Count} chosen  ·  {prompt.Detail}" : $"{_pickedModes.Count} chosen"
                : $"{_pickedModes.Count} of {prompt.Count} chosen{(prompt.MayRepeat ? "  ·  the same mode may be chosen more than once" : string.Empty)}")
            + (searched ? $"  ·  type to find one of {prompt.Modes.Count}" : string.Empty));

        string Label(int mode)
        {
            int times = _pickedModes.Count(picked => picked == mode);
            return times == 0 ? prompt.Modes[mode] : $"×{times}  {prompt.Modes[mode]}";
        }

        bool Pressable(int mode) =>
            prompt.Eligible.Contains(mode)
            && (oneClick || _pickedModes.Count < prompt.Most || (!prompt.MayRepeat && _pickedModes.Contains(mode)));

        void Press(int mode)
        {
            if (oneClick)
            {
                Send(client, new ModesChosenAnswer { Modes = [mode] });
                return;
            }

            if (!prompt.MayRepeat && _pickedModes.Remove(mode))
            {
                _needsRedraw = true;
                return;
            }

            if (_pickedModes.Count < prompt.Most)
            {
                _pickedModes.Add(mode);
                _needsRedraw = true;
            }
        }

        if (searched)
        {
            AddOptionSearch(prompt, Label, Pressable, Press);
        }
        else
        {
            for (int i = 0; i < prompt.Modes.Count; i++)
            {
                int mode = i;
                AddButton(Label(mode), () => Press(mode), enabled: Pressable(mode));
            }
        }

        if (!oneClick)
        {
            if (_pickedModes.Count > 0)
            {
                AddButton("Clear", () =>
                {
                    _pickedModes.Clear();
                    _needsRedraw = true;
                });
            }

            AddButton(
                "Done",
                () => Send(client, new ModesChosenAnswer { Modes = [.. _pickedModes] }),
                enabled: _pickedModes.Count >= prompt.Count && _pickedModes.Count <= prompt.Most,
                primary: true);
        }
    }

    /// <summary>
    /// A long list of options found by typing, as a card name is: a box, then a button for each of the best few matches
    /// (<see cref="OptionFilter"/>), changed as the typing changes without rebuilding the dock, so the box keeps its
    /// caret. Enter presses the one option the typing settles on. Options already picked stay in view.
    /// </summary>
    private void AddOptionSearch(ChooseModesPromptMessage prompt, Func<int, string> label, Func<int, bool> pressable, Action<int> press)
    {
        FlowPanel panel = _buttons;
        var field = new TextBox
        {
            PlaceholderText = "Type to find one",
            Text = _selection.NameTyped,
            Width = 220,
            FontSize = 13,
        };

        panel.Children.Add(field);
        List<UIElement> shown = [];

        void Pressed(int mode)
        {
            press(mode);
            RedrawNow();
        }

        void Show()
        {
            foreach (UIElement old in shown)
            {
                panel.Children.Remove(old);
            }

            shown.Clear();
            List<int> matches = OptionFilter.Matches(prompt.Modes, prompt.Eligible, _selection.NameTyped);
            List<int> listed = [.. _pickedModes.Distinct().Where(mode => !matches.Take(OptionsShown).Contains(mode)), .. matches.Take(OptionsShown)];

            foreach (int mode in listed)
            {
                var button = new DockButton(label(mode), primary: false, enabled: pressable(mode));
                button.Clicked += () => Pressed(mode);
                shown.Add(button);
            }

            int more = matches.Count - Math.Min(matches.Count, OptionsShown);
            if (matches.Count == 0 || more > 0)
            {
                shown.Add(new SymbolText(13, Theme.FgMuted)
                {
                    Text = matches.Count == 0 ? "Nothing matches" : $"{more} more  ·  keep typing",
                    VerticalAlignment = VerticalAlignment.Center,
                });
            }

            int at = panel.Children.IndexOf(field) + 1;
            foreach (UIElement element in shown)
            {
                panel.Children.Insert(at++, element);
            }
        }

        Show();

        field.TextChanged += (_, _) =>
        {
            _selection.NameTyped = field.Text;
            Show();
        };

        field.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter
                && OptionFilter.Settled(prompt.Modes, prompt.Eligible, _selection.NameTyped) is int settled
                && pressable(settled))
            {
                Pressed(settled);
            }

            // Typing is not a hotkey: Space in an option must not pass priority.
            e.Handled = true;
        };
    }

    private static string Spelled(int count) =>
        count switch { 1 => "one", 2 => "two", 3 => "three", _ => count.ToString(System.Globalization.CultureInfo.InvariantCulture) };

    /// <summary>A player chosen, not targeted (CR 608.2d, 614.12a): one button per eligible seat.</summary>
    private void FillChoosePlayer(GameClient client, ChoosePlayerPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "A spell";
        string subject = client.Object(prompt.Subject)?.Name ?? "it";
        string action = prompt.Verb switch
        {
            "Draw" => prompt.Amount == 1 ? "draws a card" : $"draws {prompt.Amount} cards",
            "Gift" => "is promised the gift",
            "LookAtHand" => "shows you their hand, then you choose a card name",
            "ChooseRevealedCard" => "chooses which revealed card goes into your hand; the other is exiled with a silver counter",
            "GainControl" => "gains control of this artifact",
            "GainControlOf" => $"gains control of {subject}",
            "SeparatePiles" => $"separates the {(prompt.Amount == 1 ? "revealed card" : $"{prompt.Amount} revealed cards")} into two piles",
            _ => prompt.Verb,
        };

        // Assist (CR 702.132a): the player chosen may pay any of the spell's generic mana; any other player, not only an opponent.
        if (prompt.Verb == "Assist")
        {
            Prompt($"{source}: choose another player to help pay for it", "They may pay any amount of its generic mana  ·  you pay the rest");
        }
        else if (prompt.Verb == "Protection")
        {
            Prompt($"{source}: choose a player", "This creature has protection from the chosen player.");
        }
        else if (prompt.Verb == "Protect")
        {
            // CR 310.9a, 310.12a: a battle's protector, chosen as it enters, or again when it has none (CR 704.5x).
            Prompt($"{source}: choose who protects it", "Its protector defends it, and can't attack it; anyone else facing them may.");
        }
        else
        {
            Prompt($"{source}: choose an opponent", $"The player you choose {action}");
        }

        foreach (int seat in prompt.Candidates)
        {
            AddButton(NameOf(client, seat), () => Send(client, new PlayerChosenAnswer { Player = seat }), primary: seat == prompt.Candidates[0]);
        }
    }

    /// <summary>
    /// Who takes the first turn (CR 103.1), asked of the seat chosen at random before any hand is dealt: play first or
    /// draw first, or in a bigger game which player goes first.
    /// </summary>
    private void FillChooseStartingPlayer(GameClient client, ChooseStartingPlayerPromptMessage prompt)
    {
        List<int> others = [.. prompt.Candidates.Where(seat => seat != prompt.Player)];

        Prompt(
            "You choose who goes first",
            others.Count == 1
                ? "Play first and skip your first draw, or draw first and let your opponent start"
                : "Play first, or choose who does; turns go on in seating order from them");

        AddButton("Play First", () => Send(client, new PlayerChosenAnswer { Player = prompt.Player }), primary: true);

        if (others.Count == 1)
        {
            AddButton("Draw First", () => Send(client, new PlayerChosenAnswer { Player = others[0] }));
            return;
        }

        foreach (int seat in others)
        {
            AddButton($"{NameOf(client, seat)} plays first", () => Send(client, new PlayerChosenAnswer { Player = seat }));
        }
    }

    /// <summary>Keep the opening hand or take a mulligan (CR 103.5); the hand itself is in the hand row below.</summary>
    private void FillMulligan(GameClient client, MulliganPromptMessage prompt)
    {
        int hand = client.Zone(ZoneKindView.Hand, client.Seat).Count;
        int bottom = Math.Max(0, hand - prompt.Keeps);

        Prompt(
            prompt.Mulligans switch
            {
                0 => "Your opening hand",
                1 => "Your hand after a mulligan",
                _ => $"Your hand after {prompt.Mulligans} mulligans",
            },
            bottom == 0
                ? "Keep it, or shuffle it away for a new hand"
                : $"Keeping it puts {(bottom == 1 ? "a card" : $"{bottom} cards")} of your choice on the bottom of your library");

        AddButton($"Keep {prompt.Keeps}", () => Send(client, new MayChosenAnswer { Accept = false }), primary: true);
        AddButton("Mulligan", () => Send(client, new MayChosenAnswer { Accept = true }));
    }

    /// <summary>One or more colors chosen as a spell or ability resolves (CR 202.2b).</summary>
    private void FillChooseColors(GameClient client, ChooseColorsPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "An ability";
        string many = prompt.Count switch { 1 => "a color", 2 => "two colors", 3 => "three colors", _ => $"{prompt.Count} colors" };
        string doing = prompt.Verb switch
        {
            "AddMana" when prompt.Count == 1 => "One mana of that color is added",
            "AddMana" => $"One mana of each is added  ·  {_pickedColors.Count} of {prompt.Count} chosen",
            "GainProtection" when prompt.Count == 1 => "That creature gains protection from it until end of turn",
            // A text change (CR 612.1): the word replaced, then the word replacing it.
            "ReplaceWord" => "Every instance of it in the target's text is replaced",
            "ReplacingWord" => "It replaces every instance of the first in the target's text",
            _ when prompt.Count == 1 => "Chosen, not targeted",
            _ => $"{_pickedColors.Count} of {prompt.Count} chosen",
        };
        string title = prompt.Verb switch
        {
            "ReplaceWord" => $"{source}: choose the color word to replace",
            "ReplacingWord" => $"{source}: choose the color word that replaces it",
            _ => $"{source}: choose {many}",
        };

        Prompt(title, doing);

        foreach (ColorView color in prompt.Choices)
        {
            ColorView chosen = color;
            int times = _pickedColors.Count(picked => picked == (int)chosen);
            string label = $"{{{CardActions.Symbol(chosen)}}}";

            if (prompt.Count == 1)
            {
                AddButton(label, () => Send(client, new ColorsChosenAnswer { Colors = [chosen] }));
                continue;
            }

            AddButton(
                times == 0 ? label : $"×{times}  {label}",
                () =>
                {
                    if (!prompt.MayRepeat && _pickedColors.Remove((int)chosen))
                    {
                        _needsRedraw = true;
                        return;
                    }

                    if (_pickedColors.Count < prompt.Count)
                    {
                        _pickedColors.Add((int)chosen);
                        _needsRedraw = true;
                    }
                },
                enabled: _pickedColors.Count < prompt.Count || (!prompt.MayRepeat && times > 0));
        }

        if (prompt.Count <= 1)
        {
            return;
        }

        if (_pickedColors.Count > 0)
        {
            AddButton("Clear", () =>
            {
                _pickedColors.Clear();
                _needsRedraw = true;
            });
        }

        AddButton(
            "Done",
            () => Send(client, new ColorsChosenAnswer { Colors = [.. _pickedColors.Select(picked => (ColorView)picked)] }),
            enabled: _pickedColors.Count == prompt.Count,
            primary: true);
    }

    /// <summary>
    /// A card name chosen as a permanent enters (CR 614.12a, 201.4) or as a spell or ability resolves, worded with the
    /// kind of card the name must be (CR 201.4a: "a land card name"). The hand being looked at is laid out to click; any
    /// other name is typed, and the button names whatever the typing settles on. Only the names offered can be chosen.
    /// </summary>
    private void FillNameCard(GameClient client, NameCardPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "This permanent";
        bool looking = prompt.Owner > 0;
        string owner = looking ? NameOf(client, prompt.Owner) : string.Empty;
        // What the card asks for, in its words: "a nonland card name", "a card name other than a basic land card name".
        string kind = prompt.Kind.Length > 0 ? prompt.Kind : "a card name";

        Prompt(
            $"{source}: choose {kind}",
            !looking ? $"Type {kind}"
            : prompt.Hand.Count == 0 ? $"{owner}'s hand is empty  ·  type any name"
            : $"Looking at {owner}'s hand  ·  click a card, or type any name");

        if (looking)
        {
            OpenChoices($"{owner}'s hand", closable: false, Math.Max(1, prompt.Hand.Count));

            if (prompt.Hand.Count == 0)
            {
                ShowChoicesEmpty("Their hand is empty.");
            }

            foreach (ViewHandle handle in prompt.Hand)
            {
                if (client.Object(handle) is not { } card)
                {
                    continue;
                }

                string name = card.Name;
                bool nameable = prompt.Names.BinarySearch(name, StringComparer.Ordinal) >= 0;
                AddChoice(card, new CardLook(Actionable: nameable, Clicked: nameable ? () => Send(client, new CardNamedAnswer { Name = name }) : null));
            }
        }

        var field = new TextBox
        {
            PlaceholderText = "Card name",
            Text = _selection.NameTyped,
            Width = 220,
            FontSize = 13,
        };

        _buttons.Children.Add(field);

        string? Match() => CardNameMatch(prompt.Names, _selection.NameTyped);

        DockButton naming = AddButton(
            Match() is { } matched ? $"Name {matched}" : "Name a card",
            () =>
            {
                if (Match() is { } chosen)
                {
                    Send(client, new CardNamedAnswer { Name = chosen });
                }
            },
            enabled: Match() is not null,
            primary: true);

        field.TextChanged += (_, _) =>
        {
            _selection.NameTyped = field.Text;
            string? matched = Match();
            naming.Caption = matched is null ? "Name a card" : $"Name {matched}";
            naming.Enabled = matched is not null;
        };

        field.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter && Match() is { } chosen)
            {
                Send(client, new CardNamedAnswer { Name = chosen });
                RedrawNow();
            }

            // Typing is not a hotkey: Space in a name must not pass priority.
            e.Handled = true;
        };

        if (prompt.Suggestion.Length > 0)
        {
            AddButton($"Name {prompt.Suggestion}", () => Send(client, new CardNamedAnswer { Name = prompt.Suggestion }));
        }
    }

    private static string? CardNameMatch(IReadOnlyList<string> names, string typed)
    {
        string text = typed.Trim();

        if (text.Length == 0)
        {
            return null;
        }

        if (names.FirstOrDefault(n => string.Equals(n, text, StringComparison.OrdinalIgnoreCase)) is { } exact)
        {
            return exact;
        }

        List<string> starting = [.. names.Where(n => n.StartsWith(text, StringComparison.OrdinalIgnoreCase)).Take(2)];

        if (starting.Count > 0)
        {
            return starting.Count == 1 ? starting[0] : null;
        }

        List<string> containing = [.. names.Where(n => n.Contains(text, StringComparison.OrdinalIgnoreCase)).Take(2)];
        return containing.Count == 1 ? containing[0] : null;
    }

    /// <summary>
    /// Which of several replacement effects applies first (CR 616.1). The caption says what the event is where the table
    /// words it ("3 damage from Lightning Bolt to you": damage, whom it would now be dealt to), and the detail what happens
    /// next: every other effect that still applies to the changed event then applies too, each once (CR 614.5, 616.1f), so
    /// that with two redirections the one chosen first passes the damage on to the other. Sources that share a name are told
    /// apart by what can be seen of them: the damage marked on each, and whether it is tapped.
    /// </summary>
    private void FillChooseReplacement(GameClient client, ChooseReplacementPromptMessage prompt)
    {
        // What the event is where the table words it ("You would gain 3 life", "3 damage from Lightning Bolt to you"), else
        // the object it is about.
        string subject = prompt.Event.Length > 0 ? prompt.Event : client.Object(prompt.Subject)?.Name ?? "it";

        Prompt(
            $"{subject}: which applies first?",
            $"{prompt.Sources.Count} effects want to change this  ·  then each other one that still applies does, once");

        // A source this seat holds no handle to - one that has left, seen by a page reloaded since - is named by the prompt.
        List<string> names = [.. prompt.Sources.Select((source, i) => client.Object(source)?.Name
            ?? (i < prompt.SourceNames.Count && prompt.SourceNames[i].Length > 0 ? prompt.SourceNames[i] : "An effect"))];
        for (int i = 0; i < prompt.Sources.Count; i++)
        {
            int index = i;
            string source = names[i];
            if (names.Count(name => name == source) > 1 && client.Object(prompt.Sources[i]) is { } shared)
            {
                List<string> seen = [];
                if (shared.DamageMarked > 0)
                {
                    seen.Add($"{shared.DamageMarked} damage marked");
                }

                if (shared.Tapped)
                {
                    seen.Add("tapped");
                }

                source = seen.Count > 0 ? $"{source} ({string.Join(", ", seen)})" : source;
            }

            string hint = i < prompt.Hints.Count ? prompt.Hints[i] : string.Empty;

            AddButton(
                hint.Length > 0 ? $"{source}: {hint}" : source,
                () => Send(client, new ReplacementChosenAnswer { Index = index }),
                primary: index == 0);
        }
    }

    // --- Triggers ---

    /// <summary>
    /// Which of your simultaneous triggers goes on the stack next (CR 603.3b), in a Stack Triggers window: each clicked
    /// to put it on the stack, where it stands over what is there until the order is sent with the last click.
    /// </summary>
    private void FillOrderTriggers(GameClient client, OrderTriggersPromptMessage prompt)
    {
        List<int> left = [.. _selection.TriggersLeft(prompt.Sources.Count)];

        Prompt(
            _triggerOrder.Count == 0
                ? $"{prompt.Sources.Count} of your abilities triggered"
                : $"{left.Count} still to put on the stack",
            "click them in order  ·  the last one on resolves first");

        OpenChoices("Stack Triggers", closable: false, left.Count);

        foreach (int index in left)
        {
            AddChoice(TriggerStandIn(client, prompt, index), new CardLook(Actionable: true, Clicked: () => PlaceTrigger(client, prompt, index)));
        }

        if (left.Count > 1)
        {
            AddButton("Stack All", () => StackTriggersAsShown(client, prompt), primary: true).Tooltip =
                $"Put all {left.Count} on the stack in the order shown, first one here first.\n"
                + "The last one shown goes on last, on top, and resolves first."
                + Suffix(PassKey);
        }

        if (_triggerOrder.Count > 0)
        {
            AddButton("Start over", () =>
            {
                _triggerOrder.Clear();
                _needsRedraw = true;
            });
        }
    }

    private void PlaceTrigger(GameClient client, OrderTriggersPromptMessage prompt, int index)
    {
        if (_triggerOrder.Contains(index))
        {
            return;
        }

        _triggerOrder.Add(index);
        _needsRedraw = true;

        if (_triggerOrder.Count == prompt.Sources.Count)
        {
            SendTriggerOrder(client, prompt);
        }
    }

    private void StackTriggersAsShown(GameClient client, OrderTriggersPromptMessage prompt)
    {
        List<int> order = _selection.TriggerOrderAsShown(prompt.Sources.Count);
        _triggerOrder.Clear();
        _triggerOrder.AddRange(order);
        SendTriggerOrder(client, prompt);
    }

    /// <summary>The abilities put in order and sent, still standing on the stack until the server's own take their places.</summary>
    private (List<ObjectView> StandIns, HashSet<ViewHandle> StackBefore)? _landingTriggers;

    /// <summary>Sends the order now standing, and leaves the stand-ins on the stack until the real abilities arrive.</summary>
    private void SendTriggerOrder(GameClient client, OrderTriggersPromptMessage prompt)
    {
        _landingTriggers = (
            [.. _triggerOrder.Select(i => TriggerStandIn(client, prompt, i))],
            [.. client.Zone(ZoneKindView.Stack, 0).Select(item => item.Handle)]);

        var answer = new TriggersOrderedAnswer();
        answer.Order.AddRange(_triggerOrder);
        Send(client, answer);
    }

    /// <summary>A triggered ability as it will look on the stack before it is there, with a handle the server can never use.</summary>
    private static ObjectView TriggerStandIn(GameClient client, OrderTriggersPromptMessage prompt, int index) => new()
    {
        Handle = new ViewHandle(-(((client.Prompt?.RequestId ?? 0) * 64) + index + 1)),
        Zone = ZoneKindView.Stack,
        Kind = ObjectKindView.AbilityOnStack,
        Owner = client.ActingSeat,
        Controller = client.ActingSeat,
        Name = client.Object(prompt.Sources[index])?.Name ?? "An ability",
        OracleText = index < prompt.Texts.Count ? prompt.Texts[index] : string.Empty,
    };

    /// <summary>
    /// The triggered abilities put in order so far, drawn on top of the stack: those picked while the prompt is open,
    /// and after the order is sent, those still waiting for the server's own abilities to replace them.
    /// </summary>
    private List<ObjectView> StagedTriggers(GameClient client) =>
        client.Prompt?.Prompt is OrderTriggersPromptMessage prompt
            ? [.. _triggerOrder.Select(i => TriggerStandIn(client, prompt, i))]
            : _landingTriggers?.StandIns ?? [];

    /// <summary>
    /// Hands each stand-in's place to the ability the server put on the stack for it, once they have arrived: the ones
    /// new to the stack since the order was sent, in the order sent. Given up on at the next question if they never came,
    /// so a stand-in cannot linger.
    /// </summary>
    private void LandTriggers(GameClient client)
    {
        if (_landingTriggers is not { } landing)
        {
            return;
        }

        List<ObjectView> arrived =
        [
            .. client.Zone(ZoneKindView.Stack, 0)
                .Where(item => item.Kind == ObjectKindView.AbilityOnStack && !landing.StackBefore.Contains(item.Handle)),
        ];

        if (arrived.Count >= landing.StandIns.Count)
        {
            for (int i = 0; i < landing.StandIns.Count; i++)
            {
                _flights.Alias(landing.StandIns[i].Handle, arrived[i].Handle);
            }

            _landingTriggers = null;
            _needsRedraw = true;
        }
        else if (client.Prompt is not null)
        {
            _landingTriggers = null;
            _needsRedraw = true;
        }
    }

    // --- Paying mid-resolution, searching, choosing cards ---

    private void FillUnlessPays(GameClient client, UnlessPaysPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "This card";
        string subject = client.Object(prompt.Subject)?.Name ?? "it";
        string cost = prompt.CostText();

        // The card's words for what paying does ("pay {W}{B} and 2 life to return ..."), and under them where the payment
        // stands: the life part needs nothing doing, the mana part is covered or still owed.
        string question = prompt.Question.Length > 0 ? prompt.Words(prompt.Question, subject) : $"pay {cost}?";
        string title = question.Contains("{source}", StringComparison.Ordinal)
            ? question.Replace("{source}", source, StringComparison.Ordinal)
            : $"{source}: {question}";
        string standing = prompt.LifeShort
            ? $"you no longer have the {prompt.Life} life it takes"
            : _actions.Pay is not null
                ? prompt.Total is "{0}" or "" ? "" : $"{prompt.Total} is covered"
                : _actions.Any
                    ? $"{prompt.Remaining} still to pay  ·  tap a land"
                    : $"{prompt.Remaining} still to pay  ·  nothing untapped can pay it";

        // Decaying Soil's card, gone from the graveyard since the trigger: paying is still the choice, but buys nothing.
        string detail = prompt.Subject == ViewHandle.None && prompt.Question.Contains("{subject}", StringComparison.Ordinal)
            ? "That card has left where it was, so paying gets nothing"
            : prompt.Words(prompt.Detail, subject);

        // "You may pay {X}" (CR 107.3f): X is chosen here, as it is paid, and the payment is of the X shown.
        // "You may pay {G} up to three times" (CR 603.12a): how many times is chosen here too, and the payment is the cost
        // that many times over.
        string chosen = prompt.ChoosingX ? $"X = {prompt.X}, so {prompt.Total}  ·  − and + change X"
            : prompt.ChoosingTimes ? $"{Times(prompt.X)}, so {prompt.Total}  ·  − and + change how many times{(prompt.MostTimes > 0 ? $" (up to {Spelled(prompt.MostTimes)})" : string.Empty)}"
            : "";

        Prompt(title, string.Join("  ·  ", new[] { detail, chosen, standing }.Where(part => part.Length > 0)));

        if (_actions.Pay is { } pay)
        {
            AddButton(prompt.Yes.Length > 0 ? prompt.Words(prompt.Yes, subject) : $"Pay {cost}", () => Send(client, AnswerFor(pay)), primary: true);
        }

        if (_actions.Decline is { } decline)
        {
            AddButton(prompt.No.Length > 0 ? prompt.Words(prompt.No, subject) : "Don't pay", () => Send(client, AnswerFor(decline)), primary: _actions.Pay is null);
        }

        if (prompt.ChoosingX)
        {
            _buttons.Children.Add(Stepper(
                8,
                XStep("−", "Lower X by one", _actions.DecreaseX),
                new SymbolText(13, Theme.Fg, semibold: true) { Text = $"X = {prompt.X}" },
                XStep("+", "Raise X by one", _actions.IncreaseX)));
        }
        else if (prompt.ChoosingTimes)
        {
            _buttons.Children.Add(Stepper(
                8,
                XStep("−", "Pay it one time fewer", _actions.DecreaseX),
                new SymbolText(13, Theme.Fg, semibold: true) { Text = Times(prompt.X) },
                XStep("+", "Pay it one time more", _actions.IncreaseX)));
        }

        static string Times(int n) => n == 1 ? "once" : $"{n} times";

        DockButton XStep(string text, string tooltip, ActionOptionMessage? option)
        {
            var button = new DockButton(text, primary: false, enabled: option is not null) { Tooltip = tooltip };
            if (option is not null)
            {
                button.Clicked += () => { Send(client, AnswerFor(option)); RedrawNow(); };
            }

            return button;
        }
    }

    /// <summary>Whether a library search shows only the cards it can take (the choice window's "Show only valid choices").</summary>
    private bool _searchValidOnly = true;

    /// <summary>A library search (CR 701.19), in the choice window, since a library is not on the board.</summary>
    private void FillSearch(GameClient client, SearchLibraryPromptMessage prompt)
    {
        // Whose library: the searcher's own, or another player's (Deadly Cover-Up searches its owner's), named so the
        // player never reads "your library" of someone else's.
        string library = prompt.Owner == 0 || prompt.Owner == client.Seat
            ? "your library"
            : $"{client.View?.Players.FirstOrDefault(player => player.Seat == prompt.Owner)?.Name ?? $"seat {prompt.Owner}"}'s library";
        string title = $"Search {library}";
        string destination = prompt.Destination switch
        {
            SearchDestinationView.TopOfLibrary => $"to put on top of {library}",
            SearchDestinationView.Graveyard => "to put into your graveyard",
            SearchDestinationView.Battlefield => "to put onto the battlefield",
            SearchDestinationView.BattlefieldTapped => "to put onto the battlefield tapped",
            SearchDestinationView.CastFree => "to cast without paying its mana cost",
            SearchDestinationView.Found => string.Empty,
            SearchDestinationView.LandTappedElseHand => "a land card to put onto the battlefield tapped, any other into your hand",
            SearchDestinationView.Exile => "to exile",
            SearchDestinationView.ExileFaceDown => "to exile face down",
            SearchDestinationView.Cast => "to cast as the effect resolves",
            _ => "to put into your hand",
        };
        string take = prompt.Destination switch
        {
            SearchDestinationView.Exile or SearchDestinationView.ExileFaceDown => "Exile",
            SearchDestinationView.Cast or SearchDestinationView.CastFree => "Cast",
            SearchDestinationView.Battlefield or SearchDestinationView.BattlefieldTapped => "Put onto the battlefield",
            _ => "Take",
        };

        // Where they go, said after the cards; a search that says only what it looks for (Transmute Artifact) says what follows,
        // and one whose card goes one of two ways by its type (Archdruid's Charm) says both.
        string what = prompt.Destination switch
        {
            SearchDestinationView.Found => "  ·  what becomes of it is decided once it is found",
            SearchDestinationView.LandTappedElseHand => $"  ·  {destination}",
            _ => $" {destination}",
        };

        // The cards it looks for, as its rule words them: "a basic land card", "up to 2 cards with different names".
        string noun = prompt.Kind.Length > 0 ? $"{prompt.Kind} card" : "card";
        string asks = string.Concat(prompt.Words.Select(words => " " + words));
        Prompt(
            title,
            prompt.Count == 1
                ? $"Choose {("aeiou".Contains(noun[0], StringComparison.OrdinalIgnoreCase) ? "an" : "a")} {noun}{asks}{what}  ·  {_picked.Count} chosen"
                : prompt.AnyNumber
                ? $"Choose any number of {noun}s{asks}{what}  ·  {_picked.Count} chosen"
                : $"Choose {(prompt.Optional ? "up to " : string.Empty)}{prompt.Count} {noun}s{asks}{what}  ·  {_picked.Count} chosen");

        // The cards the search may find at all, and what the rules say of the ones picked so far: which may join them, and
        // whether they are an answer - asked of the host as the picks change.
        HashSet<int> eligible = [.. prompt.Verdict.Addable.Select(card => card.Value)];
        SelectionVerdictView? judged = CardChoicePresentation.Judged(client, prompt, _picked);
        HashSet<int> addable = [.. (judged?.Addable ?? []).Select(card => card.Value)];
        List<ObjectView> shown =
        [
            .. prompt.Library
                .Where(card => !_searchValidOnly || eligible.Contains(card.Value))
                .Select(client.Object)
                .OfType<ObjectView>(),
        ];

        // In the library's order unless the player sorts them, and in one run unless they group them; the window is sized
        // by rows, so a grouped library counts each section's last row whole, and its headings.
        IReadOnlyList<CardGroup> groups = CardArrangement.Arrange(shown, _searchSort, _searchGrouping);
        bool grouped = _searchGrouping != CardGrouping.None;
        int slots = grouped
            ? groups.Sum(group => (group.Cards.Count + ChoiceColumns - 1) / ChoiceColumns * ChoiceColumns)
            : shown.Count;
        float headings = grouped ? groups.Count * (SearchHeadingHeight + ChoiceGap) : 0f;

        OpenChoices(title, closable: false, Math.Max(1, slots), viewControls: true, headings: headings);
        ShowPicked(client, prompt.Count);

        if (shown.Count == 0)
        {
            ShowChoicesEmpty($"Nothing in {library} can be taken.");
        }

        foreach (CardGroup group in groups)
        {
            if (grouped)
            {
                AddChoiceSection(group.Heading, group.Cards.Count);
            }

            foreach (ObjectView card in group.Cards)
            {
                int handle = card.Handle.Value;
                bool canTake = eligible.Contains(handle);
                // With one to find, a click trades, so any card it may find may be clicked; else only one the rules let join.
                bool joins = prompt.Count == 1 || addable.Contains(handle);

                AddChoice(card, new CardLook(
                    Selected: _picked.Contains(handle),
                    Dimmed: !canTake,
                    Actionable: canTake && !_picked.Contains(handle) && _selection.CanPick(prompt.Count) && joins,
                    Clicked: canTake && (_picked.Contains(handle) || _selection.CanPick(prompt.Count) && joins) ? () => Pick(handle, prompt.Count) : null));
            }
        }

        AddButton(
            _picked.Count == 0 ? take : _picked.Count == 1 ? $"{take} 1 card" : $"{take} {_picked.Count} cards",
            () => Send(client, new CardsChosenAnswer { Cards = [.. _picked.Select(h => new ViewHandle(h))] }),
            enabled: (_picked.Count > 0 || !prompt.Optional) && judged?.Complete == true,
            primary: true);

        if (prompt.Optional)
        {
            // CR 701.23b: a search for a stated quality may fail to find.
            AddButton("Take nothing", () => Send(client, new CardsChosenAnswer()));
        }
    }

    /// <summary>
    /// An exact number of permanents picked on the board, its last pick made: sent once the host has called it an answer,
    /// and left for the dock if it does not.
    /// </summary>
    private bool _answerWhenJudged;

    /// <summary>Choosing cards as a spell resolves: on the board where they are, or in the choice window.</summary>
    private void FillChooseCards(GameClient client, ChooseCardsPromptMessage prompt)
    {
        CardChoicePresentation model = CardChoicePresentation.Describe(client, prompt, _picked);
        if (_answerWhenJudged && CardChoicePresentation.Judged(client, prompt, _picked) is { } judged)
        {
            _answerWhenJudged = false;
            if (judged.Complete && model.AnswersOnLastPick && _picked.Count == prompt.Count)
            {
                Send(client, new CardsChosenAnswer { Cards = [.. _picked.Select(h => new ViewHandle(h))] });
                return;
            }
        }

        Prompt(model.Title, model.Detail);

        if (model.BoardZone is null)
        {
            OpenChoices(model.TrayTitle, closable: false, prompt.Cards.Count);
            HashSet<int> eligible = [.. prompt.Verdict.Addable.Select(card => card.Value)];

            foreach (ViewHandle card in prompt.Cards)
            {
                int handle = card.Value;
                bool canPick = eligible.Contains(handle);

                if (client.Object(card) is { } shown)
                {
                    AddChoice(shown, new CardLook(
                        Selected: _picked.Contains(handle),
                        Dimmed: !canPick,
                        // A card the question's rule would refuse with those picked ("with total mana value N or less", CR 202.3) is
                        // not offered: the host's word on the cards picked.
                        Actionable: canPick && !_picked.Contains(handle) && _selection.CanPick(prompt.Count) && CardChoicePresentation.Fits(client, prompt, _picked, handle),
                        Clicked: canPick && (_picked.Contains(handle) || _selection.CanPick(prompt.Count) && CardChoicePresentation.Fits(client, prompt, _picked, handle))
                            ? () => Pick(handle, prompt.Count)
                            : null,
                        Annotation: PutBackAnnotation(prompt, handle)));
                }
            }
        }

        // An exact number of permanents answers itself on the last click: no button.
        if (!model.AnswersOnLastPick)
        {
            AddButton(
                model.Confirm,
                () => Send(client, new CardsChosenAnswer { Cards = [.. _picked.Select(h => new ViewHandle(h))] }),
                enabled: model.CanSubmit,
                primary: true);
        }
    }

    // --- May ---

    /// <summary>A "you may" choice as a spell or ability resolves (CR 608.2d).</summary>
    private void FillMay(GameClient client, MayPromptMessage prompt)
    {
        string source = client.Object(prompt.Source)?.Name ?? "A spell";

        void Ask(string title, string detail, string yes, string no, bool yesIsPrimary = true)
        {
            Prompt(title, detail);
            AddButton(yes, () => Send(client, new MayChosenAnswer { Accept = true }), primary: yesIsPrimary);
            AddButton(no, () => Send(client, new MayChosenAnswer { Accept = false }));
        }

        string Named(string fallback) => client.Object(prompt.Source)?.Name ?? fallback;

        // A card that words its own question: its title, and buttons named for what each answer does.
        if (prompt.Question.Length > 0)
        {
            string title = prompt.Question.Contains("{source}", StringComparison.Ordinal)
                ? prompt.Question.Replace("{source}", source, StringComparison.Ordinal)
                : $"{source}: {prompt.Question}";

            // Garnet asks once per Saga, and two Sagas can share a name: say how many lore counters this one has.
            if (prompt.Verb == "RemoveLoreCounter"
                && client.Object(prompt.Source)?.Counters.FirstOrDefault(counter => counter.Name == "lore") is { } lore)
            {
                title += $" ({lore.Count} lore counters)";
            }

            Ask(title, prompt.Detail, prompt.Yes, prompt.No, yesIsPrimary: !prompt.NoDefault);
            return;
        }

        switch (prompt.Verb)
        {
            // Ad Nauseam: Amount is the life the card just revealed cost, not a price for the next one.
            case "RevealAgain":
                Ask(
                    $"{source}: reveal another card?",
                    $"The last one cost {prompt.Amount} life  ·  you lose life equal to the next one's mana value",
                    "Reveal another",
                    "Stop",
                    yesIsPrimary: false);
                return;

            // Obeka (CR 724.1): asked of the player whose turn it is, whoever tapped it. Neither button is primary, so
            // passing through prompts never ends a turn by accident.
            case "EndTurn":
                Ask(
                    $"{source}: end the turn?",
                    "Every spell and ability on the stack is exiled  ·  the rest of the turn is skipped, straight to cleanup",
                    "End the turn",
                    "Don't end it",
                    yesIsPrimary: false);
                return;

            // Ripple (CR 702.60a): Amount is how many cards would be revealed, four or the whole of a shorter library.
            case "RevealForRipple":
                Ask(
                    $"{source}: reveal the top {(prompt.Amount == 1 ? "card" : $"{prompt.Amount} cards")} of your library?",
                    "Cast any with the spell's name for free  ·  the rest go on the bottom",
                    "Reveal",
                    "Don't reveal");
                return;

            case "DiscardUnlessCountered":
                Ask($"Discard a card, or {source} is countered?", "Ward", "Discard a card", "Don't pay");
                return;

            // Assist (CR 702.132a): asked as the spell is cast, once its total cost is locked in. Neither button is primary, so
            // passing through the announcement never hands part of a payment to another player by accident.
            case "Assist":
                Ask(
                    $"{Named("This spell")}: choose another player to help pay for it?",
                    "They may make mana and pay any amount of its generic mana  ·  then you may make mana, and you pay the rest",
                    "Ask for help",
                    "Pay it yourself",
                    yesIsPrimary: false);
                return;

            // Miracle (CR 702.94a): neither button is primary, so spamming Pass can never reveal a card.
            case "RevealForMiracle":
                Ask(
                    $"Reveal {Named("that card")} for its miracle cost?",
                    "Every player sees it from now on, even if you then decide not to cast it",
                    "Reveal",
                    "Keep it hidden",
                    yesIsPrimary: false);
                return;

            // Gift (CR 702.174a): an optional additional cost, announced as the spell is cast. Neither button is primary,
            // so passing through the announcement never gives an opponent anything by accident.
            // Each kind of gift (CR 702.174d-i) is named for what it gives, so the button says what the opponent gets.
            case "PromiseGiftFood" or "PromiseGiftCard" or "PromiseGiftTappedFish" or "PromiseGiftExtraTurn"
                or "PromiseGiftTreasure" or "PromiseGiftOctopus":
                (string gift, string effect) = prompt.Verb switch
                {
                    "PromiseGiftFood" => ("a Food", "create a Food token"),
                    "PromiseGiftCard" => ("a card", "draw a card"),
                    "PromiseGiftTappedFish" => ("a tapped Fish", "create a tapped 1/1 blue Fish creature token"),
                    "PromiseGiftExtraTurn" => ("an extra turn", "take an extra turn after this one"),
                    "PromiseGiftTreasure" => ("a Treasure", "create a Treasure token"),
                    _ => ("an Octopus", "create an 8/8 blue Octopus creature token"),
                };
                // An instant or sorcery gives it as it resolves, before its other effects; a permanent, when it enters.
                bool instantOrSorcery = client.Object(prompt.Source) is { } cast
                    && cast.CardTypes.Any(type => type is CardTypeView.Instant or CardTypeView.Sorcery);
                string when = instantOrSorcery ? "before the spell's other effects" : "when it enters";
                Ask(
                    $"{Named("This spell")}: promise an opponent {gift}?",
                    $"They {effect} {when}, and the card does more if you do",
                    $"Gift {gift}",
                    "No gift",
                    yesIsPrimary: false);
                return;

            case "CastForMiracleCost":
                Ask($"Cast {Named("that card")} for its miracle cost?", "You pay the miracle cost instead of its mana cost", "Cast", "Don't cast");
                return;

            // Madness (CR 702.35a): the discarded card waits in exile; declining sends it to the graveyard.
            case "CastForMadnessCost":
                Ask($"Cast {Named("that card")} for its madness cost?", "You pay the madness cost instead of its mana cost. If you don't, it is put into your graveyard", "Cast", "Don't cast");
                return;

            // Rebound (CR 702.88a): the spell waits in exile; declining leaves it there for good.
            case "rebound.cast":
                Ask($"Cast {Named("that card")} without paying its mana cost?", "Rebound: if you don't, it stays exiled and you won't get another chance", "Cast", "Don't cast");
                return;

            // A Siege's defeat (CR 310.12b): cast back face up, for nothing.
            case "CastTransformed":
                Ask($"Cast {Named("that card")} transformed without paying its mana cost?", "It is cast as its back face. If you don't, it stays in exile", "Cast", "Don't cast");
                return;

            case "CastWithoutPaying":
                Ask($"Cast {Named("that card")} without paying its mana cost?", "If you don't, it goes to the bottom of your library", "Cast", "Don't cast");
                return;

            // Paradigm (CR 702.192a, 707.12): the card offered is a copy, made for this one cast.
            case "CastCopyWithoutPaying":
                Ask($"Cast a copy of {Named("that card")} without paying its mana cost?", "The card stays where it is  ·  if you don't cast the copy, no copy is made", "Cast", "Don't cast");
                return;

            case "CastDiscarded":
                Ask($"Cast {Named("that card")} without paying its mana cost?", "If you don't, it stays in your graveyard", "Cast", "Don't cast");
                return;

            case "PlayLandDuringResolution" or "CastDuringResolution" or "CastFoundOrPutIntoHand":
                ObjectView? seen = client.Object(prompt.Source);
                string card = seen switch
                {
                    { Name.Length: > 0 } => seen.Name,
                    { FaceDownName.Length: > 0 } => seen.FaceDownName,
                    _ => "that card",
                };

                if (prompt.Verb == "PlayLandDuringResolution")
                {
                    Ask($"Play {card}?", "It uses one of your land plays this turn. If you don't, it stays where it is", "Play", "Don't play");
                    return;
                }

                if (prompt.Verb == "CastFoundOrPutIntoHand")
                {
                    Ask($"Cast {card} without paying its mana cost?", "If you don't, it goes into your hand", "Cast", "Don't cast");
                    return;
                }

                Ask(
                    prompt.Amount == 1 ? $"Cast {card} without paying its mana cost?" : $"Cast {card}?",
                    prompt.Amount == 1 ? "If you don't, it stays where it is" : "You pay its costs. If you don't, it stays where it is",
                    "Cast",
                    "Don't cast");
                return;

            case "CastExiled":
                Ask($"Cast {Named("that card")}?", "You pay its costs. If you don't, it stays in exile", "Cast", "Don't cast");
                return;

            case "PutOnTop":
                Ask($"{source}: put your spell on top of your library?", "If you don't, it goes on the bottom", "Top", "Bottom");
                return;

            case "PutOnBottom":
                Ask($"Put {Named("that card")} on the bottom of its owner's library?", "If you don't, it stays on top", "Bottom", "Leave on top");
                return;

            case "PutCountersOrCast":
                Ask($"{source}: put a +1/+1 counter on it?", "If you don't, you may cast the exiled card this turn, with mana of any type", "Put a counter", "Cast it instead");
                return;

            case "CopySpell":
                Ask($"{source}: copy it?", "The copy is yours, and you may choose a new target for it", "Copy it", "Don't copy");
                return;

            case "PutIntoGraveyard":
                Ask($"Put {Named("that card")} into your graveyard?", "If you don't, it stays on top of your library", "Graveyard", "Leave on top");
                return;

            case "PayEnergy":
                Ask($"{source}: pay an energy counter?", $"{prompt.Amount} energy left. Keep paying, or stop here", "Pay 1 energy", "Stop paying");
                return;

            case "PayEnergyCost":
                Ask($"{source}: pay {prompt.Amount} energy?", $"You have the counters. Spend {prompt.Amount}, or pay nothing", $"Pay {prompt.Amount} energy", "Don't pay");
                return;

            case "RemoveACounterToDestroy":
                Ask($"{source}: remove a counter from it?", "If you don't, the permanent it is aimed at is not destroyed", "Remove a counter", "Keep the counter");
                return;

            case "BeginOnBattlefield":
                Ask($"Begin the game with {Named("that card")} on the battlefield?", "If you don't, it stays in your opening hand", "Put it out", "Keep it in hand");
                return;

            case "ProtectFromColorless":
                Ask($"{source}: protection from colorless?", "If you don't, you choose a color instead", "From colorless", "Choose a color");
                return;

            case "RevealUntilCreature":
                Ask(
                    $"{source}: reveal until you reveal a creature card?",
                    "That creature goes onto the battlefield; everything else revealed goes to your graveyard",
                    "Reveal",
                    "Don't reveal");
                return;

            case "RevealUntilSharesCreatureType":
                Ask(
                    $"{source}: reveal until you reveal a creature card that shares a creature type?",
                    "That card goes into your hand; the rest go on the bottom of your library in a random order",
                    "Reveal",
                    "Don't reveal");
                return;

            case "ChooseOddOrEven":
                Ask($"{source}: choose odd or even.", "Spells your opponents cast with that mana value will trigger it", "Odd", "Even");
                return;

            // Amount is how many graveyard cards would go; with none, a yes still shuffles (CR 701.24c-d).
            case "ShuffleIntoLibrary":
                Ask(
                    prompt.Amount switch
                    {
                        <= 0 => $"{source}: shuffle your library?",
                        1 => $"{source}: shuffle the targeted card into your library?",
                        _ => $"{source}: shuffle the {prompt.Amount} targeted cards into your library?",
                    },
                    prompt.Amount <= 0 ? "No card from your graveyard goes in" : "From your graveyard. If you don't, they stay there",
                    "Shuffle",
                    "Don't shuffle");
                return;

            case "HaveControllerDraw":
                Ask($"{source}: let its controller draw a card?", HaveControllerDrawDetail(prompt.Amount), "They draw", "Mill them instead");
                return;

            // Pia's Revolution: asked of the opponent it targeted; yes costs them damage, no gives its controller a card.
            case "TakeDamageInsteadOfReturn":
                Ask(
                    $"{source}: take {prompt.Amount} damage?",
                    "If you don't, the artifact card returns to its owner's hand",
                    $"Take {prompt.Amount} damage",
                    "Let it return",
                    yesIsPrimary: false);
                return;

            default:
                break;
        }

        string action = prompt.Verb switch
        {
            "Attach" => "attach it to that creature",
            "ReturnAndAttach" => "return it to the battlefield attached to that creature",
            "SearchLibrary" or "SearchLibraryToHand" => "search your library",
            "CreateTreasure" => "create a Treasure token",
            "CreateToken" or "CreateTokens" => "create the token",
            "Draw" => prompt.Amount == 1 ? "draw a card" : $"draw {prompt.Amount} cards",
            "ExileAndReturn" => "exile it, then return it to the battlefield",
            "DealDamage" => prompt.Amount == 1 ? "deal 1 damage to it" : $"deal {prompt.Amount} damage to it",
            "Destroy" => "destroy it",
            "Untap" => "untap it",
            "ReturnToHand" => "return it to its owner's hand",
            "Mill" => prompt.Amount == 1 ? "mill a card" : $"mill {prompt.Amount} cards",
            _ => prompt.Verb,
        };

        // An ability waiting on the stack carries the sentence it was printed with, so that is the hint whenever there is one.
        string detail = client.Object(prompt.Source) is { OracleText.Length: > 0 } asking
            ? asking.OracleText
            : string.Empty;

        // A plain "you may" a card has not worded: the card's own text is all there is to say what yes does.
        if (prompt.Verb is "Optional" or "")
        {
            Ask(source, detail, "Yes", "No");
            return;
        }

        Ask($"{source}: {action}?", detail, "Yes", "No");
    }

    private static string HaveControllerDrawDetail(int mill) => mill switch
    {
        <= 0 => "If you don't, they mill nothing and you lose no life",
        1 => "If you don't, they mill 1 card and you lose life equal to its mana value",
        _ => $"If you don't, they mill {mill} cards and you lose life equal to their total mana value",
    };

    // --- Combat ---

    /// <summary>The attacks chosen so far: each creature sent somewhere, against what it was sent at.</summary>
    private List<AttackMessage> DeclaredAttacks() =>
    [
        .. _attacking.Select(pair => new AttackMessage
        {
            Attacker = new ViewHandle(pair.Key),
            TargetPlayer = pair.Value.Player,
            TargetPermanent = pair.Value.Permanent,
        }),
    ];

    /// <summary>
    /// The attack Pass Turn declares: the creatures chosen, though the attack was not locked in - one picked up and
    /// not yet sent anywhere at the first thing it may attack, seats first - and any creature that must attack and was
    /// left home (CR 701.15b), as the default would send it.
    /// </summary>
    private AttackersDeclaredAnswer PassTurnAttack(DeclareAttackersPromptMessage prompt)
    {
        List<AttackMessage> attacks = DeclaredAttacks();
        foreach (int creature in _pending)
        {
            if (!_attacking.ContainsKey(creature) && prompt.OfferedFor(new ViewHandle(creature)) is [var first, ..])
            {
                attacks.Add(first);
            }
        }

        var required = (AttackersDeclaredAnswer)prompt.Default();
        attacks.AddRange(required.Attacks.Where(attack => attacks.All(chosen => chosen.Attacker != attack.Attacker)));
        return new AttackersDeclaredAnswer { Attacks = attacks };
    }

    /// <summary>Declares attackers, each against a defender of its own (CR 508.1), clicked on the battlefield.</summary>
    private void FillAttackers(GameClient client, DeclareAttackersPromptMessage prompt)
    {
        List<AttackMessage> declared = DeclaredAttacks();

        // What the rules make of the attack so far (CR 508.1a-d), the host's word: asked the first time it is declared, and the
        // button waits for it. Why it may not be declared - a creature an effect requires to attack left home or sent elsewhere
        // (goad, CR 701.15b; "attacks that player this combat if able") - is the dock's caption.
        AttackVerdictView? verdict = client.AttackVerdict(declared);
        if (verdict is null)
        {
            client.CheckAttacks(declared);
        }

        string? unmet = verdict?.Issue is { } why ? AttackIssueCaption(client, why) : null;

        // A cost to attack (CR 508.1h): what the attack chosen so far costs in all, paid once it is locked in.
        string cost = verdict?.Cost ?? prompt.TotalCostText(declared);

        Prompt(
            "Declare attackers",
            unmet is not null
                ? unmet
                : _pending.Count > 0
                    ? $"Choose what to attack ({_pending.Count} selected)"
                    : (_attacking.Count switch
                    {
                        0 => $"{prompt.Eligible.Count} can attack",
                        1 => "1 attacking",
                        _ => $"{_attacking.Count} attacking",
                    }) + AttackCostNote(client, prompt, cost));

        AddButton(
            _attacking.Count == 0 ? "No attack" : cost.Length > 0 ? $"Attack with {_attacking.Count}, pay {cost}" : $"Attack with {_attacking.Count}",
            () => Send(client, new AttackersDeclaredAnswer { Attacks = declared }),
            enabled: verdict?.Complete == true,
            primary: true).Tooltip = (cost.Length > 0 ? $"Lock in the attack, then pay {cost} to attack" : "Lock in the attack") + Suffix(PassKey);

        if (_pending.Count > 0)
        {
            AddCancelButton(PutDown);
        }

        // Everything at once, offered only while nothing has been chosen.
        if (_attacking.Count == 0 && prompt.Eligible.Count > 0 && prompt.AttackablePlayers.Count > 0
            && prompt.Eligible.All(creature => !prompt.IsForbidden(new AttackMessage { Attacker = creature, TargetPlayer = prompt.AttackablePlayers[0] })))
        {
            _attackAll = () =>
            {
                // Each at the first seat the host's verdict offers it, the first defender where it may.
                foreach (ViewHandle creature in prompt.Eligible)
                {
                    List<AttackMessage> offered = prompt.OfferedFor(creature);
                    int seat = offered.Exists(attack => attack.TargetPermanent.IsNone && attack.TargetPlayer == prompt.AttackablePlayers[0])
                        ? prompt.AttackablePlayers[0]
                        : offered.FirstOrDefault(attack => attack.TargetPermanent.IsNone)?.TargetPlayer ?? prompt.AttackablePlayers[0];
                    _attacking[creature.Value] = new AttackChoice(seat, ViewHandle.None);
                }

                _needsRedraw = true;
            };

            AddButton("Attack with all", _attackAll).Tooltip =
                "Send all " + prompt.Eligible.Count + " at " + NameOf(client, prompt.AttackablePlayers[0]) + Suffix(AttackAllKey);
        }

        if (_attacking.Count > 0)
        {
            AddButton("Clear", () =>
            {
                _attacking.Clear();
                _needsRedraw = true;
            });
        }
    }

    /// <summary>
    /// Why a declaration of attackers may not be made, as the host words it: the creature, what it breaks, and the seat or
    /// permanent where there is one ("Ursine Monstrosity must attack Bot2").
    /// </summary>
    private static string AttackIssueCaption(GameClient client, AttackIssueView issue)
    {
        string creature = client.Object(issue.Creature)?.Name ?? "A creature";
        string other = !issue.OtherPermanent.IsNone
            ? $" {client.Object(issue.OtherPermanent)?.Name ?? "a permanent"}"
            : issue.OtherPlayer != 0 ? $" {NameOf(client, issue.OtherPlayer)}" : string.Empty;
        return $"{creature} {issue.Words}{other}";
    }

    /// <summary>
    /// What attacking costs, for the attackers dock's line (CR 508.1d, 508.1h): the total of the attack chosen so far and
    /// the mana the player has for it; with nothing chosen, what attacking each defender costs a creature. Empty where
    /// nothing costs anything to attack.
    /// </summary>
    private static string AttackCostNote(GameClient client, DeclareAttackersPromptMessage prompt, string cost)
    {
        if (prompt.Costs.Count == 0)
        {
            return string.Empty;
        }

        string mana = prompt.CanPay >= 0 ? $" (you can make up to {prompt.CanPay} mana)" : string.Empty;
        if (cost.Length > 0)
        {
            return $"  ·  costs {cost} to attack{mana}";
        }

        // Each defender whose attack costs something, by what one creature attacking it costs.
        List<string> each = [];
        foreach (AttackCostMessage first in prompt.Costs.DistinctBy(c => (c.TargetPlayer, c.TargetPermanent.Value)))
        {
            string whom = first.TargetPermanent.IsNone
                ? NameOf(client, first.TargetPlayer)
                : client.Object(first.TargetPermanent)?.Name ?? "a permanent";
            each.Add($"attacking {whom} costs {first.Cost} a creature");
        }

        return $"  ·  {string.Join(", ", each)}{mana}";
    }

    private void FillBlockers(GameClient client, DeclareBlockersPromptMessage prompt)
    {
        // Creatures picked up still on their way to the attacker last clicked, as the host's verdicts allow.
        ContinueBlocking(client);

        List<BlockMessage> declared = DeclaredBlocks();

        // What the rules make of the blocks so far (CR 509.1a-c), the host's word: asked the first time they are declared, and
        // the button waits for it. Why they may not be declared - an attacker short of the blockers menace needs, a creature an
        // effect requires to block (Lure, "blocks this turn if able") - is the dock's caption, and what they cost (CR 509.1d) is
        // on the button.
        BlockVerdictView? verdict = DeclaredVerdict(client);
        string? issue = verdict?.Issue is { } why ? BlockIssueCaption(client, why) : null;

        // Creatures picked up that the attacker last clicked does not take, once the verdict says so: the dock says why.
        string? held = _blockingInto != 0 && _blockQueue.Count == 0 && _pending.Count > 0 && verdict is not null
            ? HeldCaption(client, prompt, new ViewHandle(_blockingInto))
            : null;
        int blockers = declared.Select(block => block.Blocker).Distinct().Count();

        Prompt(
            "Declare blockers",
            held
                ?? (_pending.Count > 0
                ? $"Choose an attacker to block ({_pending.Count} selected)"
                : issue ?? $"{prompt.Attackers.Count} attacking  ·  {blockers} blocking"));

        AddButton(
            (_blocking.Count == 0 ? "No blocks" : $"Block with {blockers}") + (verdict is { Cost.Length: > 0 } ? $" for {verdict.Cost}" : string.Empty),
            () => Send(client, new BlockersDeclaredAnswer { Blocks = declared }),
            enabled: verdict?.Complete == true,
            primary: true).Tooltip = "Lock in the blocks" + Suffix(PassKey);

        if (_pending.Count > 0)
        {
            AddCancelButton(PutDown);
        }

        if (_blocking.Count > 0)
        {
            AddButton("Clear", () =>
            {
                _blocking.Clear();
                _needsRedraw = true;
            });
        }
    }

    /// <summary>
    /// Why a declaration of blockers may not be made, as the host words it: the creature, what it breaks, and the other
    /// creature where there is one ("Llanowar Elves must block Llanowar Elves").
    /// </summary>
    private static string BlockIssueCaption(GameClient client, BlockIssueView issue)
    {
        string creature = client.Object(issue.Creature)?.Name ?? "A creature";
        return issue.Other.IsNone ? $"{creature} {issue.Words}" : $"{creature} {issue.Words} {client.Object(issue.Other)?.Name ?? "a creature"}";
    }

    /// <summary>
    /// Why the creatures still picked up did not go to the attacker clicked: as many blocking it as it may be blocked by
    /// (CR 509.1b), or else no legal declaration has them block it with the blocks made.
    /// </summary>
    private string HeldCaption(GameClient client, DeclareBlockersPromptMessage prompt, ViewHandle attacker)
    {
        string name = client.Object(attacker)?.Name ?? "That attacker";
        int index = prompt.Attackers.IndexOf(attacker);
        int most = prompt.MaximumFor(index);
        return _blocking.Count(pair => pair.Attacker == attacker.Value) >= most
            ? $"{name} can't be blocked by more than {Blockers(most)}"
            : $"{name} can't be blocked by {(_pending.Count == 1 ? "that creature" : "those creatures")} as well";
    }

    /// <summary>"one creature", "two creatures": how many blockers an attacker may have at most.</summary>
    private static string Blockers(int count) => count == 1 ? "one creature" : $"{count} creatures";

    /// <summary>Divides each creature's combat damage among what it may hit (CR 510.1c-d, 702.19b).</summary>
    private void FillAssignDamage(GameClient client, AssignCombatDamagePromptMessage prompt)
    {
        string problem = DivisionProblem(client, prompt);

        // Banding (CR 702.22j-k): the creature whose damage is divided is the opponent's, and it goes among this player's own
        // creatures it is blocked by or is blocking.
        bool theirs = prompt.Divisions.Count > 0 && prompt.Divisions.All(division => division.Banding);
        Prompt(
            theirs ? "Banding: divide their creature's combat damage" : "Assign combat damage",
            problem.Length > 0
                ? problem
                : theirs
                    ? prompt.Divisions.Count == 1
                        ? $"Your banding lets you divide {client.Object(prompt.Divisions[0].Source)?.Name ?? "their creature"}'s damage among your creatures it is fighting, as you choose"
                        : $"Your banding lets you divide {prompt.Divisions.Count} of their creatures' damage among your creatures they are fighting, as you choose"
                    : $"{prompt.Divisions.Count} creature{(prompt.Divisions.Count == 1 ? string.Empty : "s")} to divide  ·  each must add up to its power");

        AddButton(
            "Deal damage",
            () => Send(client, new CombatDamageAssignedAnswer
            {
                Shares =
                [
                    .. _shares
                        .Where(pair => pair.Value > 0)
                        .Select(pair => new DamageShareMessage
                        {
                            Source = new ViewHandle(pair.Key.Source),
                            Creature = new ViewHandle(pair.Key.Creature),
                            Amount = pair.Value,
                        }),
                ],
            }),
            enabled: problem.Length == 0,
            primary: true).Tooltip = "Lock in the division" + Suffix(PassKey);

        foreach (DamageDivisionMessage division in prompt.Divisions)
        {
            string source = client.Object(division.Source)?.Name ?? "A creature";
            int assigned = Assigned(division);

            _buttons.Children.Add(new SymbolText(13, assigned == division.Damage ? Theme.Fg : Theme.FgMuted, semibold: true)
            {
                Text = division.Banding ? $"{source} (theirs, banding): {assigned} of {division.Damage}" : $"{source}: {assigned} of {division.Damage}",
            });

            foreach (ViewHandle blocker in division.Creatures)
            {
                string name = client.Object(blocker)?.Name ?? "a creature";
                AddShareRow(division, blocker.Value, name, source, name);
            }

            if (division.MayAssignToDefender)
            {
                AddShareRow(division, 0, $"{NameOf(client, division.Defender)}  (trample)", source, "the defender");
            }
        }
    }

    /// <summary>
    /// A share's row: − and + beside how much of the source's damage goes to it. Each button is named for what it does and
    /// the share it changes ("More of Ulvenwald Oddity's damage to Benalish Hero (now 2)"), as a person reading it hears it.
    /// </summary>
    private void AddShareRow(DamageDivisionMessage division, int creature, string name, string source, string to)
    {
        (int Source, int Creature) key = (division.Source.Value, creature);
        int share = _shares.GetValueOrDefault(key);
        bool room = Assigned(division) < division.Damage;

        DockButton less = Step("−", share > 0, () => _shares[key] = Math.Max(0, share - 1));
        DockButton more = Step("+", room, () => _shares[key] = share + 1);
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(less, $"Less of {source}'s damage to {to} (now {share})");
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(more, $"More of {source}'s damage to {to} (now {share})");
        _buttons.Children.Add(Stepper(6, less, new SymbolText(13, Theme.Fg) { Text = $"{share}  {name}" }, more));
    }

    private int Assigned(DamageDivisionMessage division) =>
        _shares.Where(pair => pair.Key.Source == division.Source.Value).Sum(pair => pair.Value);

    /// <summary>Why the division cannot be sent yet, in words, or nothing when it can: the server's own checks (CR 510.1a, 702.19b).</summary>
    private string DivisionProblem(GameClient client, AssignCombatDamagePromptMessage prompt)
    {
        foreach (DamageDivisionMessage division in prompt.Divisions)
        {
            string source = client.Object(division.Source)?.Name ?? "A creature";
            int assigned = Assigned(division);

            if (assigned != division.Damage)
            {
                return $"{source} has {division.Damage} damage to assign, not {assigned}";
            }

            if (division.MayAssignToDefender && _shares.GetValueOrDefault((division.Source.Value, 0)) > 0)
            {
                for (int i = 0; i < division.Creatures.Count; i++)
                {
                    int lethal = i < division.Lethal.Count ? division.Lethal[i] : 0;
                    if (_shares.GetValueOrDefault((division.Source.Value, division.Creatures[i].Value)) < lethal)
                    {
                        return $"{source} must assign lethal damage to {client.Object(division.Creatures[i])?.Name ?? "each blocker"} before any reaches the player";
                    }
                }
            }
        }

        return string.Empty;
    }

    // --- The dock's own parts ---

    private void Prompt(string title, string? detail = null)
    {
        // A decision of a player this seat controls (CR 723.5) says whose it is: "you" in it is that player.
        if (_gameClient is { Prompt.ForSeat: > 0 } controlling)
        {
            title = $"Deciding for {NameOf(controlling, controlling.Prompt!.ForSeat)}  ·  {title}";
        }

        _promptTitle.Text = title;
        _promptDetail.Text = detail ?? string.Empty;
        _promptDetail.Visibility = detail is { Length: > 0 } ? Visibility.Visible : Visibility.Collapsed;
        MirrorPromptInChoices();
    }

    /// <summary>Resolve All, beside Resolve while more than one thing is waiting.</summary>
    private void AddResolveAllButton(GameClient client)
    {
        IReadOnlyList<ObjectView> stack = client.Zone(ZoneKindView.Stack, 0);
        if (stack.Count < 2)
        {
            return;
        }

        AddButton("Resolve All", () =>
        {
            _resolveAll = [.. stack.Select(item => item.Handle)];
            Send(client, new PassAnswer());
        }).Tooltip =
            $"Pass priority until all {stack.Count} things on the stack have resolved, stops included.\n" +
            "Stops passing as soon as anything else is put on the stack, by you or your opponent.";
    }

    /// <summary>A Cancel button, which the Cancel key presses.</summary>
    private void AddCancelButton(Action onPressed)
    {
        _cancel = onPressed;
        AddButton("Cancel", onPressed).Tooltip = "Cancel" + Suffix(CancelKey);
    }

    private DockButton AddButton(string text, Action onPressed, bool enabled = true, bool primary = false)
    {
        var button = new DockButton(text, primary, enabled: enabled);
        button.Clicked += () =>
        {
            onPressed();
            RedrawNow();
        };

        // The first main button on the dock is what the Pass key presses, unless the prompt said otherwise.
        if (primary && enabled)
        {
            _pass ??= onPressed;
        }

        _buttons.Children.Add(button);
        return button;
    }

    private void ShowErrors(GameClient client)
    {
        if (client.Errors.Count == 0)
        {
            _errorLabel.Text = string.Empty;
            _errorLabel.Visibility = Visibility.Collapsed;
            return;
        }

        ErrorMessage latest = client.Errors[^1];
        _errorLabel.Text = $"{latest.Code}: {latest.Detail}";
        _errorLabel.Visibility = Visibility.Visible;
    }

    private static string DescribeOutcome(GameClient client)
    {
        GameOverMessage outcome = client.Outcome!;

        if (outcome.Outcome == GameOutcomeView.Draw)
        {
            return "The game is a draw.";
        }

        return outcome.WinningTeam == (client.Me?.Team ?? 0) ? "You win." : "You lose.";
    }

    /// <summary>Where the game is, as the player would say it: "Your first main phase", "Bot 2's upkeep".</summary>
    private static string Moment(GameClient client)
    {
        if (client.View is not { } view)
        {
            return string.Empty;
        }

        string whose = view.ActivePlayer == client.Seat ? "Your" : $"{NameOf(client, view.ActivePlayer)}'s";
        return $"{whose} {StepPhrase(view.Step)}";
    }

    /// <summary>What is waiting on top of the stack, whose it is and what it is aimed at.</summary>
    private string? OnTopOfStack(GameClient client)
    {
        IReadOnlyList<ObjectView> stack = client.Zone(ZoneKindView.Stack, 0);
        if (stack.Count == 0)
        {
            return null;
        }

        ObjectView top = stack[^1];
        string whose = top.Controller == client.Seat ? "you" : NameOf(client, top.Controller);
        string what = top.Kind switch
        {
            ObjectKindView.SpellCopy => $"Copy of {top.Name}",

            // Named after its source, as the log names it. How it got there is only known from the announcement, so an
            // ability put on before this client was watching is just an ability.
            ObjectKindView.AbilityOnStack => top.Name.Length > 0 ? $"{AbilityKind(top)} of {top.Name}" : AbilityKind(top),
            _ => top.Name,
        };

        return top.Targets.Count == 0
            ? $"{what} controlled by {whose}"
            : $"{what} controlled by {whose}, at {Aimed(client, top.Targets)}";
    }

    private string AbilityKind(ObjectView ability) => _abilitiesTriggered.TryGetValue(ability.Handle, out bool triggered)
        ? triggered ? "Triggered ability" : "Activated ability"
        : "Ability";

    private static string Aimed(GameClient client, IReadOnlyList<SpellTargetView> targets)
    {
        List<string> named = [];

        foreach (SpellTargetView target in targets)
        {
            string name = target.IsPlayer
                ? target.Player == client.Seat ? "you" : NameOf(client, target.Player)
                : client.Object(target.Object)?.Name ?? "something gone";

            named.Add(target.StillLegal ? name : $"{name} (no longer legal)");
        }

        return named.Count == 1
            ? named[0]
            : $"{string.Join(", ", named.Take(named.Count - 1))} and {named[^1]}";
    }

    /// <summary>What the priority prompt is offering, counted, not counting lands' own mana abilities.</summary>
    private static string Available(GameClient client, PriorityPromptMessage priority)
    {
        HashSet<int> lands = [];
        HashSet<int> spells = [];
        HashSet<int> abilities = [];
        HashSet<int> faceDown = [];
        HashSet<int> plottable = [];
        HashSet<int> suspendable = [];
        HashSet<int> foretellable = [];
        HashSet<int> unlockable = [];
        var companion = false;

        foreach (ActionOptionMessage option in priority.Options)
        {
            switch (option.Kind)
            {
                case ActionKindView.PlayLand:
                    lands.Add(option.Source.Value);
                    break;

                case ActionKindView.CastSpell:
                    spells.Add(option.Source.Value);
                    break;

                case ActionKindView.ActivateAbility
                    when client.Object(option.Source)?.CardTypes.Contains(CardTypeView.Land) != true:
                    abilities.Add(option.Source.Value);
                    break;

                case ActionKindView.TurnFaceUp:
                    faceDown.Add(option.Source.Value);
                    break;

                case ActionKindView.TakeCompanion:
                    companion = true;
                    break;

                case ActionKindView.Plot:
                    plottable.Add(option.Source.Value);
                    break;

                case ActionKindView.Suspend:
                    suspendable.Add(option.Source.Value);
                    break;

                case ActionKindView.Foretell:
                    foretellable.Add(option.Source.Value);
                    break;

                case ActionKindView.UnlockDoor:
                    unlockable.Add(option.Source.Value);
                    break;

                default:
                    break;
            }
        }

        List<string> parts = [];

        if (companion)
        {
            parts.Add("Take your companion");
        }

        if (lands.Count > 0)
        {
            parts.Add("Play a land");
        }

        if (spells.Count > 0)
        {
            parts.Add(spells.Count == 1 ? "Cast a spell" : $"Cast a spell ({spells.Count} castable)");
        }

        if (abilities.Count > 0)
        {
            parts.Add(abilities.Count == 1 ? "Activate an ability" : $"Activate an ability ({abilities.Count})");
        }

        if (faceDown.Count > 0)
        {
            parts.Add(faceDown.Count == 1 ? "Turn a card face up" : $"Turn a card face up ({faceDown.Count})");
        }

        if (plottable.Count > 0)
        {
            parts.Add(plottable.Count == 1 ? "Plot a card" : $"Plot a card ({plottable.Count})");
        }

        if (suspendable.Count > 0)
        {
            parts.Add(suspendable.Count == 1 ? "Suspend a card" : $"Suspend a card ({suspendable.Count})");
        }

        if (foretellable.Count > 0)
        {
            parts.Add(foretellable.Count == 1 ? "Foretell a card" : $"Foretell a card ({foretellable.Count})");
        }

        if (unlockable.Count > 0)
        {
            parts.Add(unlockable.Count == 1 ? "Unlock a door" : $"Unlock a door ({unlockable.Count})");
        }

        return parts.Count == 0 ? "Nothing to play" : string.Join("  ·  ", parts);
    }

    private static string StepPhrase(TurnStepView step) => step switch
    {
        TurnStepView.Untap => "untap step",
        TurnStepView.Upkeep => "upkeep",
        TurnStepView.Draw => "draw step",
        TurnStepView.PrecombatMain => "first main phase",
        TurnStepView.BeginCombat => "beginning of combat",
        TurnStepView.DeclareAttackers => "declare attackers step",
        TurnStepView.DeclareBlockers => "declare blockers step",
        TurnStepView.CombatDamage => "combat damage step",
        TurnStepView.EndOfCombat => "end of combat",
        TurnStepView.PostcombatMain => "second main phase",
        TurnStepView.End => "end step",
        TurnStepView.Cleanup => "cleanup step",
        _ => "turn",
    };

    private static string NameOf(GameClient client, int seat)
    {
        if (seat == 0)
        {
            return "nobody";
        }

        return client.View?.Players.FirstOrDefault(player => player.Seat == seat)?.Name ?? $"seat {seat}";
    }
}
