using System.Text.RegularExpressions;
using Cantrip.Art;
using Cantrip.Client;
using Cantrip.Protocol;
using Cantrip.Server.Host;
using Cantrip.Server.Rules;
using Microsoft.Playwright;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// Plays a <c>.cantrip</c> scenario with Bot1's seat a browser page (docs/scenarios-in-the-browser.md): a real table plays
/// the script, says for each of Bot1's questions what the script means it to answer, and this clicks its way there as a
/// person would - the card, the target, the row of a menu, the mana in the pool, the button - with real mouse events at
/// the places the page publishes. The table fails the scenario at the line when the page answers otherwise. The checks of
/// what the client shows (<c>Client.</c> keys) are judged here against the page.
/// <para>
/// The page is set up so that nothing answers for the player but the player: auto-mana off (a menu asks the colour),
/// every step stopped at and priority held, so the page never passes by itself where the script means an action.
/// </para>
/// </summary>
public sealed class ScenarioPlayer
{
    /// <summary>Every step of either player's turn, so the page asks the player at every priority.</summary>
    private static readonly string EveryStep = string.Join(
        ",",
        (TurnStepView[])[TurnStepView.Upkeep, TurnStepView.Draw, TurnStepView.PrecombatMain, TurnStepView.BeginCombat, TurnStepView.DeclareAttackers,
            TurnStepView.DeclareBlockers, TurnStepView.CombatDamage, TurnStepView.EndOfCombat, TurnStepView.PostcombatMain, TurnStepView.End]);

    private static readonly TimeSpan Patience = TimeSpan.FromSeconds(90);

    private readonly TablePage _page;
    private readonly ScenarioTable _table;
    private readonly List<string> _trace = [];

    /// <summary>The requests this driver answered by clicking.</summary>
    private readonly HashSet<int> _clicked = [];

    /// <summary>The label of the button the last choice of cards was confirmed with; empty when the last pick answered itself.</summary>
    private string _lastConfirm = string.Empty;

    /// <summary>The rows of the card menu the page last opened, which are gone by the time a check after the click is judged.</summary>
    private string _lastMenu = string.Empty;

    /// <summary>What the dock said while the last card action's targets were being picked: its title and its line, or nothing.</summary>
    private string _lastTargeting = string.Empty;

    /// <summary>A zone opened to click a card whose menu is open: shut once its row is chosen.</summary>
    private (int Seat, bool Graveyard)? _zoneToClose;

    /// <summary>Whether the page holds priority, as it does unless it is given the stops a player would set.</summary>
    private readonly bool _hold;

    /// <summary>Bot2's seat as a client of the table's, answering as the script means after a pause; null when the script answers it.</summary>
    private readonly (GameClient Client, TimeSpan Thinks)? _opponent;

    /// <summary>
    /// The mana source an X is raised with as it is announced, and how many of them: clicked on the battlefield rather than
    /// the dock's plus, planned for X and tapped by the page once the payment is asked. Null to step X on the dock.
    /// </summary>
    private readonly (string Name, int Count)? _planX;

    /// <summary>
    /// Whether Bot1's page is a phone at an in-person table (<c>?inperson=phone</c>, docs/in-person.md): tapped rather than
    /// clicked, and every card it does not show - the battlefield, which lies on the tablets - and every player tapped on a
    /// tablet instead, through the page's stand-in for the relay (<c>globalThis.cantripTableTaps</c>, MainPage.Test.cs).
    /// </summary>
    private readonly bool _phone;

    /// <summary>The request the table asks Bot1 now: the one a tap on a tablet is routed to.</summary>
    private int _asking;

    /// <summary>The taps made on the tablets so far, which the phone numbers as it takes them.</summary>
    private int _tableTaps;

    /// <summary>
    /// Whether the phone's first tap on a tablet is preceded by taps it must not use (<see cref="ProbeRefusalsAsync"/>); set
    /// until they have been made.
    /// </summary>
    private bool _probeRefusals;

    /// <summary>
    /// The in-person table Bot1's phone plays at for real, or null: its taps on the tablets made by a real finger on a real
    /// tablet, through the relay, rather than queued on the phone (<see cref="InPersonTable"/>).
    /// </summary>
    private readonly InPersonTable? _atTable;

    private ScenarioPlayer(TablePage page, ScenarioTable table, bool hold, (GameClient Client, TimeSpan Thinks)? opponent, (string Name, int Count)? planX, bool phone, bool probeRefusals, InPersonTable? atTable = null)
    {
        _atTable = atTable;
        _phone = phone;
        _probeRefusals = probeRefusals;
        _planX = planX;
        _page = page;
        _table = table;
        _hold = hold;
        _opponent = opponent;
    }

    /// <summary>
    /// Plays the scenario at <paramref name="path"/> to its end. Null when it passed, page and table alike; else what
    /// failed, with the table's report or the page's, and the last steps taken. <paramref name="stopsMine"/> and
    /// <paramref name="stopsTheirs"/> are the steps the page stops in on Bot1's turn and on Bot2's, every step (and
    /// priority held) unless given: a test of the stops themselves gives the ones a player would set, does not hold
    /// priority, and the page passes by itself everywhere else, the table judging each such pass as it judges a click.
    /// With <paramref name="opponentThinks"/>, Bot2 is a client at the table too, as an opponent at a real table is, and
    /// answers each of its questions as the script means that long after it is asked. With <paramref name="planX"/>, an X is
    /// raised by planning that many of the named mana source for it, and the page pays with them by itself. With
    /// <paramref name="phone"/>, Bot1's page is a phone at an in-person table (<see cref="_phone"/>); with
    /// <paramref name="probeRefusals"/> too, its first tap on a tablet follows three it must not use (<see cref="ProbeRefusalsAsync"/>).
    /// </summary>
    public static async Task<string?> PlayAsync(
        WebClient web,
        string path,
        IReadOnlyList<TurnStepView>? stopsMine = null,
        IReadOnlyList<TurnStepView>? stopsTheirs = null,
        TimeSpan? opponentThinks = null,
        (string Name, int Count)? planX = null,
        bool phone = false,
        bool probeRefusals = false,
        InPersonTable? atTable = null)
    {
        phone |= atTable is not null;
        ScenarioSpec scenario = ScenarioSpec.Load(path, opponentThinks is null ? [1] : [1, 2]);
        await using ScenarioTable table = await ScenarioTable.StartAsync(scenario, WebClient.Content, rules: atTable?.Rules, decks: atTable?.Decks);
        await using Microsoft.AspNetCore.Builder.WebApplication art = await ArtServer.StartAsync(
            new ArtStore(new ArtStoreOptions { ContentRoot = WebClient.Content, CacheDirectory = WebClient.ArtCache }), "127.0.0.1", 0);

        // At a real in-person table the phone is served by the table's own site, beside its relay, and takes its seat - at
        // the table and at the relay - with the seat's ticket, as its QR code would give it.
        (Uri site, string seat) = atTable is null ? (web.Site, "&seat=1&name=Bot1") : await atTable.SeatAsync(table, art.Urls.First());
        await using TablePage page = await TablePage.OpenAsync(
            web.Browser,
            site,
            $"server={Uri.EscapeDataString(table.Address.AbsoluteUri)}&art={Uri.EscapeDataString(art.Urls.First())}{(phone ? "&inperson=phone" : string.Empty)}{seat}",
            WebClient.Pictures(Path.Combine(nameof(ScenarioPlayer), scenario.Name + (atTable is not null ? $"-{atTable.Name}" : phone ? (planX is null ? "-phone" : "-phone-planned") : string.Empty))),
            settings: new Dictionary<string, string> { ["auto-mana"] = "false" },
            storage: new Dictionary<string, string>
            {
                ["cantrip-stops-mine"] = stopsMine is null ? EveryStep : string.Join(",", stopsMine),
                ["cantrip-stops-theirs"] = stopsTheirs is null ? EveryStep : string.Join(",", stopsTheirs),
            },
            phone: phone);

        GameClient? opponent = null;
        if (opponentThinks is not null)
        {
            opponent = new GameClient(await WebSocketTransport.ConnectAsync(table.Address));
            opponent.Join("Bot2", 2);
        }

        var player = new ScenarioPlayer(
            page,
            table,
            hold: stopsMine is null && stopsTheirs is null,
            opponent is null ? null : (opponent, opponentThinks!.Value),
            planX,
            phone,
            probeRefusals,
            atTable);
        try
        {
            return await player.PlayAsync();
        }
        catch (Exception error) when (error is TimeoutException or Xunit.Sdk.XunitException or PlaywrightException or NotDrivenException)
        {
            await page.PictureAsync("failed");
            return player.Failure(error.Message);
        }
        finally
        {
            opponent?.Disconnect();
        }
    }

    /// <summary>Bot2's client given the question the table asks it, and the answer the script means sent after its pause.</summary>
    private async Task AnswerAsOpponentAsync(ScenarioExpected expected)
    {
        (GameClient client, TimeSpan thinks) = _opponent!.Value;
        DateTime giveUp = DateTime.UtcNow + Patience;
        while (client.Prompt?.RequestId != expected.RequestId)
        {
            if (DateTime.UtcNow > giveUp)
            {
                throw new TimeoutException($"Bot2's client never got request {expected.RequestId}.");
            }

            if (client.Pump() == 0)
            {
                await Task.Delay(5);
            }
        }

        if (expected.Undo || expected.Answer is null)
        {
            throw new Xunit.Sdk.XunitException($"Bot2's client does not undo ({expected.Line})");
        }

        await Task.Delay(thinks);
        client.Answer(expected.Answer);
        _trace.Add($"Bot2 {expected.Line ?? "(default)"}: {Describe(expected)}");
    }

    private async Task<string?> PlayAsync()
    {
        bool holding = !_hold;
        while (true)
        {
            ScenarioProgress progress = await _table.NextAsync(Patience);
            if (progress.Expected is { Seat: 2 } theirs && _opponent is not null)
            {
                await AnswerAsOpponentAsync(theirs);
                continue;
            }

            if (progress.Expected is { Seat: not 1 })
            {
                continue;
            }

            // The page is brought up to the question asked (or to the end) before anything on it is judged.
            if (progress.Expected is { } waiting)
            {
                _trace.Add($"table asks request {waiting.RequestId}");
            }

            // A page past the question answered it itself (a pass it would make for anyone): the table judged that answer
            // as it judges a click, so there is nothing to do but note it. It may have nothing new to be asked yet, as
            // after its last pass of the scenario, so having answered the question by itself counts as past it - not
            // one this driver answered, which an undo or a payment step asks again under the same request.
            TableState state = progress.Expected is { } asked
                ? await _page.SettledAsync(s => s.RequestId >= asked.RequestId || AnsweredItself(s, asked))
                : await _page.SettledAsync();

            if (!holding)
            {
                await _page.ClickAsync("Hold Priority", "toggle");
                holding = true;
                state = await _page.SettledAsync();
            }

            foreach (ScenarioCheck check in progress.Checks)
            {
                if (await JudgeAsync(check, state) is { } wrong)
                {
                    await _page.PictureAsync("check-failed");
                    return Failure(wrong);
                }
            }

            if (progress.Verdict is { } verdict)
            {
                if (!verdict.Passed)
                {
                    return Failure(verdict.Report ?? "the table failed the scenario");
                }

                foreach (ScenarioCheck check in verdict.Client)
                {
                    if (await JudgeAsync(check, state) is { } wrong)
                    {
                        await _page.PictureAsync("check-failed");
                        return Failure(wrong);
                    }
                }

                return _page.Errors.Count > 0 ? Failure("the page threw: " + string.Join(" | ", _page.Errors)) : null;
            }

            if (progress.Expected is { } passed && (state.RequestId > passed.RequestId || AnsweredItself(state, passed)))
            {
                _trace.Add($"{passed.Line ?? "(default)"}: {Describe(passed)}, answered by the page itself");
                continue;
            }

            if (progress.Expected is { } expected)
            {
                _trace.Add($"{expected.Line ?? "(default)"}: {Describe(expected)}");
                _clicked.Add(expected.RequestId);
                _asking = expected.RequestId;

                // A phone's layout is pictured at every question it answers, since no other test shows it.
                if (_phone)
                {
                    await _page.PictureAsync($"phone-request-{expected.RequestId}");
                }

                if (_atTable is not null)
                {
                    await _atTable.CheckAsync(state);
                }

                await AnswerAsync(expected, state);
            }
        }
    }

    /// <summary>Whether the page, holding no question, answered this one by itself rather than by this driver's click.</summary>
    private bool AnsweredItself(TableState state, ScenarioExpected asked) =>
        state.RequestId == 0 && state.AnsweredRequest >= asked.RequestId && !_clicked.Contains(asked.RequestId);

    private string Failure(string message) =>
        $"{message}\n\nBot1's last answers, clicked:\n  {string.Join("\n  ", _trace.TakeLast(15))}"
        + (_page.Errors.Count > 0 ? "\n\nThe page threw: " + string.Join(" | ", _page.Errors) : string.Empty);

    private static string Describe(ScenarioExpected expected) => expected.Undo
        ? "Undo"
        : expected.Answer switch
        {
            TakeActionAnswer action => $"{action.Kind} {action.Source.Value}{(action.Color is { } c ? " " + c : string.Empty)}",
            { } answer => answer.GetType().Name,
            null => "nothing",
        };

    // --- Answers, as clicks ---

    private async Task AnswerAsync(ScenarioExpected expected, TableState state)
    {
        if (expected.Undo)
        {
            await _page.ClickAsync("Undo", "button");
            return;
        }

        switch (expected.Answer)
        {
            case PassAnswer:
                await _page.Page.Keyboard.PressAsync("Space");
                return;

            case TakeActionAnswer action:
                await TakeAsync(action, state, expected.Line);
                return;

            // X, or the times kicked, stepped up from the 0 it is asked at on the dock's plus, which sends nothing, and announced
            // with Okay (CR 601.2b).
            case AnnounceAnswer when _planX is { } plan && _phone:
                foreach (int source in await PhoneBoardHandlesAsync(card => card.Place == "battlefield" && card.Name == plan.Name && !card.Tapped && card.Annotation != "For X", plan.Count))
                {
                    await TableTapAsync($"o{source}");
                }

                await _page.ClickAsync("Okay", "button");
                return;

            case AnnounceAnswer announced when _planX is { } plan:
                for (int i = 0; i < plan.Count; i++)
                {
                    TableState now = await _page.SettledAsync();
                    CardState source = now.Cards.FirstOrDefault(card => card.Place == "battlefield" && card.Name == plan.Name && !card.Tapped && card.Annotation != "For X")
                        ?? throw new NotDrivenException($"No {plan.Name} left to plan for X", now);
                    await _page.TapAsync(source.X, source.Y);
                }

                await _page.ClickAsync("Okay", "button");
                return;

            case AnnounceAnswer announced:
                for (int i = 0; i < announced.Value; i++)
                {
                    await _page.ClickAsync("+", "button");
                }

                await _page.ClickAsync("Okay", "button");
                return;

            case AttackersDeclaredAnswer attack:
                foreach (AttackMessage one in attack.Attacks)
                {
                    await ClickHandleAsync(one.Attacker.Value);

                    // With a choice of what to attack (a planeswalker, a battle, another seat) the click picks the creature
                    // up, and a click on the seat's box or on the permanent sends it there, as a person does.
                    if ((await _page.SettledAsync()).Selected > 0)
                    {
                        await ClickTargetAsync(new TargetMessage { Object = one.TargetPermanent, Player = one.TargetPlayer });
                    }
                }

                // The dock's button waits for the host's word on the attack declared (CheckAttacks), as it does for a person.
                await _page.SettledAsync();
                await ClickButtonStartingAsync(attack.Attacks.Count == 0 ? "No attack" : "Attack with");
                return;

            case BlockersDeclaredAnswer block:
                foreach (BlockMessage one in block.Blocks)
                {
                    await ClickHandleAsync(one.Blocker.Value);
                    await ClickHandleAsync(one.Attacker.Value);
                }

                // The dock's button waits for the host's word on the blocks declared (CheckBlocks), as it does for a person.
                await _page.SettledAsync();
                await _page.Page.Keyboard.PressAsync("Space");
                return;

            case MayChosenAnswer may:
                // A "you may" asks Yes or No; a mulligan, which is a may too, Keep or Mulligan.
                if (state.Controls.Any(c => c.Kind == "button" && c.Label == "Yes"))
                {
                    await _page.ClickAsync(may.Accept ? "Yes" : "No", "button");
                }
                else if (state.Dock is { Count: >= 2 } && state.Prompt == "MayPromptMessage")
                {
                    // A may that names what it does (Cast, Don't cast; a gift promise, "Gift a tapped Fish", "No gift"): the dock's first
                    // button is yes, its second no, whatever follows (a Cancel while a spell is announced).
                    await ClickDockAsync(may.Accept ? 0 : 1);
                }
                else
                {
                    await ClickButtonStartingAsync(may.Accept ? "Mulligan" : "Keep");
                }

                return;

            case CardsChosenAnswer chosen:
            {
                foreach (ViewHandle card in chosen.Cards)
                {
                    await ClickHandleAsync(card.Value);
                }

                // An exact count answers itself on the last click; otherwise the dock's button, named for what it does
                // with that many ("Discard 1"), or the pass key where that is the button.
                TableState after = await _page.SettledAsync();
                _lastConfirm = string.Empty;
                if (after.RequestId == state.RequestId)
                {
                    // The dock's own button (a choice window's too): named for what it does with that many ("Discard 2", "Exile it"), else the
                    // first that can be pressed that is not a cancel; the zone buttons ("Exile 1") are not it.
                    List<DockButtonState> offered = [.. after.Dock?.Where(b => b.Enabled && !b.Label.StartsWith("Cancel", StringComparison.Ordinal)) ?? []];
                    DockButtonState? named = offered.FirstOrDefault(b => b.Label.EndsWith($" {chosen.Cards.Count}", StringComparison.Ordinal))
                        ?? offered.FirstOrDefault(b => chosen.Cards.Count == 1 && b.Label.EndsWith(" it", StringComparison.Ordinal))
                        ?? offered.FirstOrDefault();
                    ControlState? confirm = named is null
                        ? after.Controls.FirstOrDefault(c => c.Kind == "button" && (c.Label.EndsWith($" {chosen.Cards.Count}", StringComparison.Ordinal) || chosen.Cards.Count == 1 && c.Label.EndsWith(" it", StringComparison.Ordinal)))
                        : after.Controls.FirstOrDefault(c => c.Kind == "button" && c.Label == named.Label);
                    if (confirm is not null)
                    {
                        _lastConfirm = confirm.Label;
                        await _page.TapAsync(confirm.X, confirm.Y);
                    }
                    else
                    {
                        await _page.Page.Keyboard.PressAsync("Space");
                    }
                }

                return;
            }

            // A choice of player: a button for each, named for the player.
            case PlayerChosenAnswer chosenPlayer:
            {
                string name = state.Players?.FirstOrDefault(p => p.Seat == chosenPlayer.Player)?.Name is { Length: > 0 } shown
                    ? shown
                    : throw new NotDrivenException($"seat {chosenPlayer.Player}, whose name the page does not publish", state);
                await ClickButtonNamedAsync(name);
                return;
            }

            case ModesChosenAnswer modes:
            {
                // A choice of modes: a button for each, named by the mode's text; one mode answers on its click, several
                // are picked and then sent with Done.
                foreach (int mode in modes.Modes)
                {
                    string label = state.Modes is { } asked && mode < asked.Count
                        ? asked[mode]
                        : throw new NotDrivenException($"mode {mode}, which the page does not list", state);
                    await ClickButtonNamedAsync(label);
                }

                TableState after = await _page.SettledAsync();
                // One mode of a plain choice answers on its click; one of a range ("one or both") is sent with Done like several.
                if (after.RequestId == state.RequestId
                    && (modes.Modes.Count != 1 || after.Controls.Any(c => c.Kind == "button" && c.Label.StartsWith("Done", StringComparison.Ordinal))))
                {
                    await ClickButtonStartingAsync("Done");
                }

                return;
            }

            case TargetsChosenAnswer targets:
            {
                foreach (TargetMessage target in targets.Targets)
                {
                    await ClickTargetAsync(target);
                }

                // A target chosen as the spell is cast, after its additional cost, is confirmed with "Done with 1 target".
                TableState after = await _page.SettledAsync();
                if (after.RequestId == state.RequestId && after.Controls.Any(c => c.Kind == "button" && c.Label.StartsWith("Done with", StringComparison.Ordinal)))
                {
                    await ClickButtonStartingAsync("Done with");
                }

                // "Up to" nothing is an answer of its own, with a button: "No target", "No choice", "Choose none".
                if (targets.Targets.Count == 0)
                {
                    ControlState none = state.Controls.FirstOrDefault(c => c.Kind == "button"
                        && (c.Label.StartsWith("No target", StringComparison.Ordinal) || c.Label.StartsWith("No choice", StringComparison.Ordinal)
                            || c.Label.StartsWith("Choose none", StringComparison.Ordinal)))
                        ?? throw new NotDrivenException("a choice of no target at all", state);
                    await _page.TapAsync(none.X, none.Y);
                }

                return;
            }

            case ColorsChosenAnswer chosenColors:
            {
                // A choice of colors (CR 105.1, 202.2b): a button for each color, named by its mana symbol ("{U}"), clicked
                // in the order chosen; more than one is sent with Done.
                foreach (ColorView color in chosenColors.Colors)
                {
                    string symbol = color switch
                    {
                        ColorView.White => "{W}",
                        ColorView.Blue => "{U}",
                        ColorView.Black => "{B}",
                        ColorView.Red => "{R}",
                        _ => "{G}",
                    };
                    await ClickButtonNamedAsync(symbol);
                }

                TableState after = await _page.SettledAsync();
                if (after.RequestId == state.RequestId && after.Controls.Any(c => c.Kind == "button" && c.Label.StartsWith("Done", StringComparison.Ordinal)))
                {
                    await ClickButtonStartingAsync("Done");
                }

                return;
            }

            case CardNamedAnswer named:
            {
                // A card name is typed into the dock's box, as a person names a card that is not in front of them, and sent
                // with the button the typing settles on ("Name Wasteland"). Only names the question allows are matched.
                ControlState box = state.Controls.FirstOrDefault(c => c.Kind == "textbox")
                    ?? throw new NotDrivenException($"the name {named.Name}, with no box to type it in", state);
                await _page.TapAsync(box.X, box.Y);
                await _page.Page.Keyboard.TypeAsync(named.Name);
                await ClickButtonNamedAsync($"Name {named.Name}");
                return;
            }

            case CardsDistributedAnswer distributed:
                await DistributeAsync(distributed, state);
                return;

            case CountersRemovedAnswer removed:
            {
                // Counters removed from among permanents as a cost (CR 118.3): the dock's button for each kind on each permanent
                // takes one more, "Take a -1/-1 counter from Sylvan Caryatid (0 of 2)", clicked that many times; then the
                // button that removes them. An effect's "remove up to three counters" (CR 608.2d) is the same, its button saying
                // how many it removes: "Remove 2 counters", or "Remove no counters" for an answer that takes none.
                foreach (CounterPoolMessage removal in removed.Removals)
                {
                    string name = state.Cards.FirstOrDefault(c => c.Handle == removal.Object.Value)?.Name
                        ?? throw new NotDrivenException($"counters on card #{removal.Object.Value}, which is not in view", state);
                    for (int i = 0; i < removal.Count; i++)
                    {
                        await ClickButtonStartingAsync($"Take a {removal.Counter} counter from {name} (");
                    }
                }

                await ClickButtonStartingAsync("Remove ");
                return;
            }

            case CombatDamageAssignedAnswer assigned:
            {
                // How creatures divide their combat damage (CR 510.1c-d, 702.22j-k): the dock seeds each share with the
                // engine's division, and each share's − and + are named for it, "Less of Ox's damage to Bear (now 1)" (the
                // player trampled over is "the defender"). Each share is stepped to what the answer gives it - none where the
                // answer leaves it out - lowering every share first so there is room to raise; then Deal damage.
                string NameOfHandle(ViewHandle handle) => state.Cards.FirstOrDefault(c => c.Handle == handle.Value)?.Name ?? string.Empty;
                int Wanted(string source, string to) => assigned.Shares
                    .Where(s => NameOfHandle(s.Source) == source && (to == "the defender" ? s.Creature.IsNone : !s.Creature.IsNone && NameOfHandle(s.Creature) == to))
                    .Sum(s => s.Amount);

                foreach (bool lowering in new[] { true, false })
                {
                    for (int guard = 0; guard < 256; guard++)
                    {
                        TableState now = await _page.SettledAsync();
                        ControlState? step = null;
                        foreach (ControlState less in now.Controls.Where(c => c.Kind == "button" && c.Label.StartsWith("Less of ", StringComparison.Ordinal)))
                        {
                            int of = less.Label.LastIndexOf("'s damage to ", StringComparison.Ordinal);
                            int at = less.Label.LastIndexOf(" (now ", StringComparison.Ordinal);
                            string source = less.Label["Less of ".Length..of];
                            string to = less.Label[(of + "'s damage to ".Length)..at];
                            int share = int.Parse(less.Label[(at + " (now ".Length)..^1], System.Globalization.CultureInfo.InvariantCulture);
                            int wanted = Wanted(source, to);
                            if (lowering && share > wanted)
                            {
                                step = less;
                                break;
                            }

                            if (!lowering && share < wanted)
                            {
                                step = now.Controls.FirstOrDefault(c => c.Kind == "button" && c.Label == "More" + less.Label["Less".Length..])
                                    ?? throw new NotDrivenException($"a + for {source}'s damage to {to}", now);
                                break;
                            }
                        }

                        if (step is null)
                        {
                            break;
                        }

                        await _page.TapAsync(step.X, step.Y);
                    }
                }

                await ClickButtonStartingAsync("Deal damage");
                return;
            }

            case TriggersOrderedAnswer ordered:
                // Which of the controller's simultaneous triggers goes on the stack next (CR 603.3b): the Stack Triggers window shows each
                // as a stand-in card whose handle is minus (the request's number times 64, plus its index, plus 1); each is clicked in
                // the order named, and the last click sends the order.
                foreach (int index in ordered.Order)
                {
                    await ClickHandleAsync(-((expected.RequestId * 64) + index + 1));
                }

                return;

            case ReplacementChosenAnswer replacement:
                // Which of several replacement effects applies first (CR 616.1): the dock has a button for each, "Source: what
                // it does", in the order the question offers them.
                await ClickDockAsync(replacement.Index);
                return;

            default:
                throw new NotDrivenException(expected.Answer?.GetType().Name ?? "nothing", state);
        }
    }

    /// <summary>
    /// Cards looked at and placed (scry, surveil, Ponder): each card still to place is shown with a button per pile that takes
    /// it, in the piles' order ("Top", "Bottom", "Graveyard"). "Top" puts the card on top of those already there, as "On top"
    /// does in the order of cards put into a graveyard together (CR 404.3), so those piles are clicked from their lowest card
    /// up; every other pile adds at its end, so in order. Then Done.
    /// </summary>
    private async Task DistributeAsync(CardsDistributedAnswer distributed, TableState state)
    {
        if (distributed.Shuffle)
        {
            await ClickButtonStartingAsync("Shuffle instead");
            return;
        }

        // The piles' buttons, in order, as the first card still to place offers them.
        HashSet<int> placed = [.. distributed.Placements.SelectMany(pile => pile).Select(card => card.Value)];
        int first = distributed.Placements.SelectMany(pile => pile).Select(card => card.Value).FirstOrDefault();
        List<string> piles = first == 0 ? [] : [.. ButtonsUnder(state, first, placed).Select(button => button.Label)];
        if (distributed.Placements.Any(pile => pile.Count > 0) && piles.Count != distributed.Placements.Count)
        {
            string cards = string.Join("; ", state.Cards.Where(c => c.Visible).Select(c => $"{c.Name} #{c.Handle} at {c.X:0},{c.Y:0} {c.W:0}x{c.H:0}"));
            string buttons = string.Join("; ", state.Controls.Where(c => c.Kind == "button").Select(c => $"'{c.Label}' at {c.X:0},{c.Y:0}"));
            throw new NotDrivenException($"a placement into {distributed.Placements.Count} piles, under a card offering {piles.Count} (cards: {cards}; buttons: {buttons})", state);
        }

        for (int i = 0; i < distributed.Placements.Count; i++)
        {
            IEnumerable<ViewHandle> order = piles[i] is "Top" or "On top" ? Enumerable.Reverse(distributed.Placements[i]) : distributed.Placements[i];
            foreach (ViewHandle card in order)
            {
                TableState now = await _page.SettledAsync();
                ControlState button = ButtonsUnder(now, card.Value, placed).FirstOrDefault(b => b.Label == piles[i])
                    ?? throw new NotDrivenException($"a \"{piles[i]}\" button under card #{card.Value}", now);
                await _page.TapAsync(button.X, button.Y);
            }
        }

        await _page.SettledAsync();
        await ClickButtonStartingAsync("Done");
    }

    /// <summary>
    /// The buttons beneath a card still to place, left to right: those just below the row of cards to place that are nearer
    /// its middle than any other card's of the row. The row is the cards being placed (`placed`), so a permanent or a spell
    /// drawn at the same height and size is not taken for one of them.
    /// </summary>
    private static List<ControlState> ButtonsUnder(TableState state, int handle, HashSet<int> placed)
    {
        List<CardState> shown = [.. state.Cards.Where(c => c.Visible && c.Handle == handle).OrderByDescending(c => c.Y)];
        if (shown.Count == 0)
        {
            throw new NotDrivenException($"card #{handle} to place, which is not in view", state);
        }

        // A card looked at may be drawn twice: in the row of cards to place, and where it is (the top of a library known to its
        // owner). The one to place is the one with the piles' buttons under it.
        foreach (CardState card in shown)
        {
            List<CardState> row = [.. state.Cards.Where(c => c.Visible && placed.Contains(c.Handle) && Math.Abs(c.Y - card.Y) < 1f && Math.Abs(c.H - card.H) < 1f)];
            List<ControlState> buttons =
            [
                .. state.Controls
                    .Where(c => c.Kind == "button" && c.Y > card.Y + (card.H / 2) && c.Y < card.Y + card.H)
                    .Where(c => row.MinBy(other => Math.Abs(other.X - c.X))?.Handle == handle)
                    .OrderBy(c => c.X),
            ];
            if (buttons.Count > 0)
            {
                return buttons;
            }
        }

        return [];
    }

    private async Task TakeAsync(TakeActionAnswer action, TableState state, string? line)
    {
        switch (action.Kind)
        {
            // A window for mana abilities while a spell with assist is cast (CR 702.132a) is closed with Done; anything else is Okay.
            case ActionKindView.ConfirmCasting:
                await _page.ClickAsync(state.Dock?.Any(b => b.Label == "Done") == true ? "Done" : "Okay", "button");
                return;

            // A payment's dock is its pay button (once the cost is covered) and its decline, each named by the card for what
            // it does ("Pay {W}{B} and 2 life", "Enters tapped", "Let it be countered"): pay is the first, decline the last.
            case ActionKindView.PayCost when state.Prompt == "UnlessPaysPromptMessage" && state.Dock is { Count: >= 2 }:
                await ClickDockAsync(0);
                return;

            case ActionKindView.PayCost:
                await ClickButtonStartingAsync("Pay");
                return;

            case ActionKindView.DeclineCost when state.Prompt == "UnlessPaysPromptMessage" && state.Dock is { Count: >= 1 } dock:
                await ClickDockAsync(dock.Count - 1);
                return;

            case ActionKindView.DeclineCost:
                await ClickButtonStartingAsync("Don't pay");
                return;

            case ActionKindView.CancelCasting:
                await _page.ClickAsync("Cancel", "button");
                return;

            // The value of X, a click at a time on the dock's plus and minus.
            case ActionKindView.IncreaseX:
                await _page.ClickAsync("+", "button");
                return;

            case ActionKindView.DecreaseX:
                await _page.ClickAsync("−", "button");
                return;

            case ActionKindView.AssignMana:
            {
                PlayerState me = state.Players?.FirstOrDefault(p => p.Seat == state.Seat)
                    ?? throw new NotDrivenException("AssignMana, with no pool shown", state);
                // Restricted mana is a chip of its own, captioned with what it is for, and so is mana a rule tells apart by
                // the permanent that produced it, captioned "from" that permanent (CR 106.3); plain mana's caption says at
                // most how much is spoken for ("1 to spell"). The answer's restriction, or its source, says which to click.
                bool restricted = action.Filter != ManaSpendFilterView.None || action.Effect != ManaSpendEffectView.None;
                bool sourced = !action.Source.IsNone;
                static bool Captioned(ManaChipState c) => c.Caption.Length > 0 && !Regex.IsMatch(c.Caption, @"^\d+ to spell$");
                static bool From(ManaChipState c) => c.Caption.StartsWith("from ", StringComparison.Ordinal);
                List<ManaChipState> ofType = [.. me.Mana.Where(c => c.Type == action.Mana?.ToString() && c.Clickable)];
                ManaChipState chip = (sourced
                        ? ofType.FirstOrDefault(From)
                        : ofType.FirstOrDefault(c => Captioned(c) == restricted && !From(c)))
                    ?? throw new Xunit.Sdk.XunitException(
                        $"No {(sourced ? "source's" : restricted ? "restricted" : "plain")} {action.Mana} in the pool to click: "
                        + string.Join(", ", me.Mana.Select(c => $"{c.Count} {c.Type}{(c.Caption.Length > 0 ? $" ({c.Caption})" : string.Empty)}")));
                await _page.TapAsync(chip.X, chip.Y);
                return;
            }

            case ActionKindView.PlayLand or ActionKindView.CastSpell or ActionKindView.ActivateAbility or ActionKindView.TurnFaceUp or ActionKindView.UnlockDoor
                or ActionKindView.TakeCompanion:
            {
                // `Menu` is the menu this action opens: none, for a card with one thing to do - never one an earlier card opened.
                _lastMenu = string.Empty;
                _lastTargeting = string.Empty;
                await ClickHandleAsync(action.Source.Value);
                TableState after = await _page.SettledAsync();

                // A known library top is also drawn on its pile. Clicking the pile opens the pane;
                // act on the actual card inside that newly opened pane, as a player does.
                if (after.RequestId == state.RequestId && !after.Controls.Any(c => c.Kind == "item")
                    && !state.Cards.Any(c => c.Visible && c.Handle == action.Source.Value && c.Place == "choice")
                    && after.Cards.FirstOrDefault(c => c.Visible && c.Handle == action.Source.Value && c.Place == "choice") is { } inPane)
                {
                    (float x, float y) = ClickPoint(after, inPane);
                    await _page.TapAsync(x, y);
                    after = await _page.SettledAsync();
                }

                // A card with more than one thing to do opens a menu: the row the answer is.
                if (after.Controls.Any(c => c.Kind == "item"))
                {
                    await ChooseRowAsync(action, after, line);
                }

                if (action.Targets.Count > 0)
                {
                    TableState targeting = await _page.SettledAsync();
                    _lastTargeting = $"{targeting.Title} / {targeting.Detail}";
                }

                foreach (TargetMessage target in action.Targets)
                {
                    await ClickTargetAsync(target);
                }

                if (action.Sacrifice is { } sacrifice)
                {
                    await ClickHandleAsync(sacrifice.Value);
                }

                return;
            }

            default:
                throw new NotDrivenException($"{nameof(TakeActionAnswer)} {action.Kind}", state);
        }
    }

    /// <summary>The menu's row for the answer: the one adding its colour, for a mana ability of any colour.</summary>
    private async Task ChooseRowAsync(TakeActionAnswer action, TableState state, string? line)
    {
        List<ControlState> rows = [.. state.Controls.Where(c => c.Kind == "item")];
        _lastMenu = string.Join("; ", rows.Select(r => r.Label));

        // Else, where the script names the ability (`Activate(Ability: +2)`, `Cast(Ability: Flashback {R})`), the row that says it.
        string? ability = line is null ? null : System.Text.RegularExpressions.Regex.Match(line, @"Ability:\s*(.+?)\)\s*$") is { Success: true } named
            ? named.Groups[1].Value.Trim()
            : null;
        List<ControlState> saying = ability is null
            ? []
            : [.. rows.Where(r => r.Label.Contains(ability.Replace("-", "−", StringComparison.Ordinal), StringComparison.OrdinalIgnoreCase)
                || r.Label.Contains(ability, StringComparison.OrdinalIgnoreCase))];

        // A card with several abilities lists them: the rows of the one the answer is. A menu of one ability's rows (a mana
        // ability's colours) is the same menu whole.
        if (rows.Any(r => r.Ability == action.AbilityIndex))
        {
            rows = [.. rows.Where(r => r.Ability == action.AbilityIndex)];
        }
        if (action.OptionId != 0)
        {
            rows = [.. rows.Where(r => r.OptionId == action.OptionId)];
        }

        ControlState? row = action.Color is { } color
            ? rows.FirstOrDefault(r => r.Label.Contains($"{{{Symbol(color)}}}", StringComparison.Ordinal))
            : DoorOf(action, state) is { } door ? rows.FirstOrDefault(r => r.Label.Contains(door, StringComparison.Ordinal))
            : rows.Count == 1 ? rows[0]
            : saying.Count == 1 ? saying[0] : null;
        if (row is null)
        {
            throw new NotDrivenException($"which row of the menu is {action.Kind}{(action.Color is { } c ? " " + c : string.Empty)}", state);
        }

        await _page.TapAsync(row.X, row.Y);
        if (_zoneToClose is { } open)
        {
            _zoneToClose = null;
            await CloseZoneAsync(open.Seat, open.Graveyard);
        }
    }

    /// <summary>
    /// The words of the menu row for the door an action is about (" — Unholy Annex" ends a cast of that half, "Unlock Ritual Chamber" starts its unlock): the half of a split card that a cast is
    /// of (the left way is the half printed first, CR 709.3), or the locked door of a Room that an unlock opens - the only
    /// locked one, or, with both locked, the left's before the right's (CR 709.5e). Null for any other action.
    /// </summary>
    private static string? DoorOf(TakeActionAnswer action, TableState state)
    {
        CardState? card = state.Cards.FirstOrDefault(c => c.Visible && c.Handle == action.Source.Value);
        if (card is null)
        {
            return null;
        }

        if (action.Kind == ActionKindView.CastSpell && action.AbilityIndex is 238 or 239)
        {
            string[] halves = card.Halves.Split("; ", StringSplitOptions.RemoveEmptyEntries);
            return halves.Length == 2 ? " — " + halves[action.AbilityIndex == 239 ? 0 : 1] : null;
        }

        if (action.Kind == ActionKindView.UnlockDoor)
        {
            string[] locked = [.. card.Doors.Split("; ", StringSplitOptions.RemoveEmptyEntries)
                .Where(door => door.EndsWith(":locked", StringComparison.Ordinal))
                .Select(door => door[..^":locked".Length])];
            return locked.Length switch { 0 => null, 1 => "Unlock " + locked[0], _ => "Unlock " + locked[action.AbilityIndex == 0 ? 0 : 1] };
        }

        return null;
    }

    private static char Symbol(ColorView color) => color switch
    {
        ColorView.White => 'W',
        ColorView.Blue => 'U',
        ColorView.Black => 'B',
        ColorView.Red => 'R',
        ColorView.Green => 'G',
        _ => '?',
    };

    private async Task ClickTargetAsync(TargetMessage target)
    {
        if (target.Object.Value != 0)
        {
            await ClickHandleAsync(target.Object.Value);
            return;
        }

        if (_phone)
        {
            await TableTapAsync($"p{target.Player}");
            return;
        }

        TableState state = await _page.SettledAsync();
        PlayerState box = state.Players?.FirstOrDefault(p => p.Seat == target.Player)
            ?? throw new Xunit.Sdk.XunitException($"Seat {target.Player} is not shown to click.");
        await _page.TapAsync(box.X, box.Y);
    }

    /// <summary>
    /// A card in a graveyard or in exile is drawn only while that zone is open: a person opens the pile, or the Exile button,
    /// and clicks it there. The zones are tried in turn - each opened, looked into, shut again if it was not there - and the
    /// card in the one that holds it is returned, its zone left open. Null when none holds it.
    /// </summary>
    private async Task<(CardState Card, int Seat, bool Graveyard)?> OpenZoneHoldingAsync(int handle)
    {
        TableState state = await _page.SettledAsync();
        List<(int Seat, bool Graveyard, float X, float Y)> openers = [];
        foreach (PlayerState player in state.Players ?? [])
        {
            // Exile, and a sideboard (the button reads "Companion" while a companion waits there, CR 702.139a).
            openers.AddRange((player.Zones ?? []).Where(z => OutsideOrExile(z.Text)).Select(z => (player.Seat, false, z.X, z.Y)));
            if (player.GraveyardX > 0)
            {
                openers.Add((player.Seat, true, player.GraveyardX, player.GraveyardY));
            }
        }

        foreach ((int seat, bool graveyard, float x, float y) in openers)
        {
            await _page.TapAsync(x, y);
            TableState open = await _page.SettledAsync();
            if (open.Cards.Where(c => c.Visible && c.Handle == handle).MaxBy(c => c.W) is { } found)
            {
                return (found, seat, graveyard);
            }

            // Not in this one: shut it, where its opener is now (the board moves over when a graveyard opens).
            await _page.TapAsync(x, y);
            await _page.SettledAsync();
        }

        return null;
    }

    /// <summary>
    /// Scrolls each open graveyard column - with the wheel, over a card of it in view - down and then up until the card is in
    /// view, as a person scrolls a long graveyard to the card they want. Null when no column brings it into view.
    /// </summary>
    private async Task<CardState?> ScrollGraveyardsToAsync(int handle)
    {
        TableState state = await _page.SettledAsync();
        List<string> places = [.. state.Cards.Where(c => c.Visible && c.Place.EndsWith("graveyard", StringComparison.Ordinal)).Select(c => c.Place).Distinct()];
        foreach (string place in places)
        {
            foreach (float delta in new[] { 120f, -120f })
            {
                for (int turn = 0; turn < 20; turn++)
                {
                    TableState now = await _page.SettledAsync();
                    if (now.Cards.Where(c => c.Visible && c.Handle == handle).MaxBy(c => c.W) is { } found)
                    {
                        return found;
                    }

                    CardState? over = now.Cards.FirstOrDefault(c => c.Visible && c.Place == place);
                    if (over is null)
                    {
                        break;
                    }

                    await _page.Page.Mouse.MoveAsync(over.X, over.Y);
                    await _page.Page.Mouse.WheelAsync(0, delta);
                }
            }
        }

        return null;
    }

    /// <summary>Shuts a zone this driver opened to click a card in it, if it is still open: a player's table is not left so.</summary>
    private async Task CloseZoneAsync(int seat, bool graveyard)
    {
        TableState state = await _page.SettledAsync();
        PlayerState? player = state.Players?.FirstOrDefault(p => p.Seat == seat);
        if (graveyard)
        {
            if (state.GraveyardsOpen && player is { GraveyardX: > 0 })
            {
                await _page.TapAsync(player.GraveyardX, player.GraveyardY);
                await _page.SettledAsync();
            }

            return;
        }

        if (player?.Zones?.FirstOrDefault(z => OutsideOrExile(z.Text) && z.Open) is { } button)
        {
            await _page.TapAsync(button.X, button.Y);
            await _page.SettledAsync();
        }
    }

    /// <summary>A zone button that opens exile or a sideboard: what a card a player clicks may be in, but for the piles.</summary>
    private static bool OutsideOrExile(string text) =>
        text.StartsWith("Exile", StringComparison.Ordinal) || text.StartsWith("Sideboard", StringComparison.Ordinal);

    private async Task ClickHandleAsync(int handle)
    {
        TableState state = await _page.SettledAsync();

        // A phone shows the hand, the stack and its choices; a card it does not show is on the battlefield, on a tablet.
        if (_phone && !state.Cards.Any(c => c.Visible && c.Handle == handle))
        {
            await TableTapAsync($"o{handle}");
            return;
        }
        // A card in a graveyard is drawn twice while the graveyards are open (the pile, and the list a player reads it in) and only
        // as the pile's top card while they are shut: a click on the pile opens or shuts the list and acts on nothing, so the card
        // is clicked in the list, which is opened as a person opens it.
        CardState? card = state.Cards.Where(c => c.Visible && c.Handle == handle).MaxBy(c => c.W);
        if (card is { Place: "" or "graveyard" or "opponent-graveyard", Row: "" } && !state.GraveyardsOpen)
        {
            card = null;
        }

        // An open graveyard holding more cards than its column shows scrolls: the card is looked for there as a person scrolls to it.
        if (card is null && state.GraveyardsOpen)
        {
            card = await ScrollGraveyardsToAsync(handle);
            if (card is not null)
            {
                state = await _page.SettledAsync();
            }
        }

        (int Seat, bool Graveyard)? opened = null;
        if (card is null && await OpenZoneHoldingAsync(handle) is { } inZone)
        {
            card = inZone.Card;
            opened = (inZone.Seat, inZone.Graveyard);
        }

        if (card is null)
        {
            throw new Xunit.Sdk.XunitException(
                $"Handle {handle} is not in view. In view: {string.Join(", ", state.Cards.Where(c => c.Visible).Select(c => $"{c.Name} #{c.Handle} ({c.Place})"))}");
        }

        // The stack fans its cards, each covering most of the one beneath: the strip of the card's left edge is the part of
        // it that is always in view.
        (float x, float y) = ClickPoint(opened is null ? state : await _page.SettledAsync(), card);
        await _page.TapAsync(x, y);
        // A click that opened a menu is not left: closing the zone would dismiss the menu whose row is about to be chosen.
        if (opened is { } zone)
        {
            if ((await _page.SettledAsync()).Controls.Any(c => c.Kind == "item"))
            {
                _zoneToClose = zone;
            }
            else
            {
                await CloseZoneAsync(zone.Seat, zone.Graveyard);
            }
        }
    }

    /// <summary>
    /// A tap on a tablet, routed to Bot1's phone for the request the table asks now, as the relay routes it: <c>o12</c> the
    /// object the seat knows as 12, <c>p2</c> player 2. The phone must use it - act on it exactly as a click there would.
    /// </summary>
    private async Task TableTapAsync(string what)
    {
        if (_probeRefusals)
        {
            _probeRefusals = false;
            await ProbeRefusalsAsync();
        }

        if (_atTable is not null)
        {
            await TapOnTabletAsync(what);
            return;
        }

        (TableTapState tap, TableState state) = await InjectTapAsync(what, _asking);
        if (!tap.Used)
        {
            throw new Xunit.Sdk.XunitException($"The phone did not use a tap on the table ({what}) for request {_asking}: \"{state.InPerson?.Refusal}\"");
        }
    }

    /// <summary>
    /// A tap made with a finger on a real tablet at the in-person table (<see cref="InPersonTable.TapAsync"/>), which the relay
    /// routes to Bot1's phone: the phone must use it. It may first be handed a tap for a question just answered, which it
    /// lets go and the relay passes on.
    /// </summary>
    private async Task TapOnTabletAsync(string what)
    {
        TableState before = await _page.SettledAsync();
        int settled = before.InPerson!.Taps.Count;
        await _atTable!.TapAsync(what, before);
        TableState state = await _page.SettledAsync(s => s.InPerson!.Taps.Skip(settled).Any(t => t.Used) || s.InPerson.Taps.Count >= settled + 4);
        List<TableTapState> taken = [.. state.InPerson!.Taps.Skip(settled)];
        _trace.Add($"tablet tap {what} for request {_asking}: {string.Join(", ", taken.Select(t => $"request {t.RequestId} {(t.Used ? "used" : "unused")}"))}");
        if (!taken.Any(t => t.Used))
        {
            throw new Xunit.Sdk.XunitException($"The phone did not use a tap on the tablet ({what}) for request {_asking}: \"{state.InPerson.Refusal}\"");
        }

        await _atTable.CheckAsync(state);
    }

    /// <summary>A tap on a tablet routed to Bot1's phone for <paramref name="request"/>, and what the phone made of it once settled.</summary>
    private async Task<(TableTapState Tap, TableState State)> InjectTapAsync(string what, int request)
    {
        int id = ++_tableTaps;
        await _page.Page.EvaluateAsync(
            "tap => { globalThis.cantripTableTaps = (globalThis.cantripTableTaps ?? '') + tap + ';'; }",
            $"{what}@{request}");
        TableState state = await _page.SettledAsync(s => s.InPerson?.Taps.Any(t => t.TapId == id) == true);
        TableTapState tap = state.InPerson!.Taps.First(t => t.TapId == id);
        _trace.Add($"table tap {what} for request {request}: {(tap.Used ? "used" : "unused")}{(tap.Held ? ", held for its prompt" : string.Empty)}");
        return (tap, state);
    }

    /// <summary>
    /// Three taps on the tablets the phone must not use, made before its first real one, each leaving the question where it
    /// was: a tap routed to a request already past, let go at once, and one routed to a request that never comes, held for
    /// its prompt about a second and then let go - neither said on the phone, since the relay passes such a tap on and the
    /// other seat may use it; then a player who is no choice in the question on screen, which the phone does say.
    /// </summary>
    private async Task ProbeRefusalsAsync()
    {
        int asking = _asking;
        TableState before = await _page.SettledAsync();

        // A tap routed to a question already past, or to one that never reaches the page, is the relay's to pass on - the
        // other seat may use it - so the phone says nothing of it.
        (TableTapState past, TableState afterPast) = await InjectTapAsync($"p{before.Seat}", asking - 1);
        Assert(!past.Used && !past.Held && afterPast.InPerson!.Refusal.Length == 0, $"a tap for a request already past was {Describe(past, afterPast)}");

        System.Diagnostics.Stopwatch waited = System.Diagnostics.Stopwatch.StartNew();
        (TableTapState early, TableState afterEarly) = await InjectTapAsync($"p{before.Seat}", asking + 1000);
        Assert(!early.Used && early.Held && waited.Elapsed >= TimeSpan.FromSeconds(0.9) && afterEarly.InPerson!.Refusal.Length == 0,
            $"a tap for a request still to come was {Describe(early, afterEarly)} after {waited.Elapsed.TotalSeconds:0.0} s");

        (TableTapState player, TableState afterPlayer) = await InjectTapAsync($"p{before.Seat}", asking);
        Assert(!player.Used && !player.Held && afterPlayer.InPerson!.Refusal.Length > 0, $"a tap on player {before.Seat}, no choice now, was {Describe(player, afterPlayer)}");

        Assert(afterPlayer.RequestId == before.RequestId && afterPlayer.AnsweredRequest == before.AnsweredRequest,
            $"the question moved under taps the phone did not use: request {before.RequestId} -> {afterPlayer.RequestId}");

        static string Describe(TableTapState tap, TableState state) =>
            $"{(tap.Used ? "used" : "unused")}, {(tap.Held ? "held" : "not held")}, the phone saying \"{state.InPerson?.Refusal}\"";

        static void Assert(bool holds, string otherwise)
        {
            if (!holds)
            {
                throw new Xunit.Sdk.XunitException(otherwise);
            }
        }
    }

    /// <summary>
    /// The handles of cards on the battlefield that a person at the table would find on a tablet: read from the phone's own
    /// Board, opened and shut again for it, since the phone does not publish the board it keeps hidden.
    /// </summary>
    private async Task<List<int>> PhoneBoardHandlesAsync(Func<CardState, bool> which, int count)
    {
        await _page.ClickAsync("Board", "button");
        TableState shown = await _page.SettledAsync(s => s.InPerson?.Board == true);
        await _page.PictureAsync($"phone-board-request-{_asking}");
        List<int> handles = [.. shown.Cards.Where(c => c.Visible).Where(which).Select(c => c.Handle).Distinct().Take(count)];
        await _page.ClickAsync("Board", "button");
        await _page.SettledAsync(s => s.InPerson?.Board == false);
        return handles.Count == count ? handles : throw new NotDrivenException($"{count} such cards on the board, with {handles.Count}", shown);
    }

    /// <summary>
    /// Where to click a card: its middle, unless cards laid over it (a graveyard's pile, a fanned hand) cover that, and then the first
    /// place on it that none of them does - as a person goes for the part of the card that shows. The stack's fan shows a strip.
    /// </summary>
    private static (float X, float Y) ClickPoint(TableState state, CardState card)
    {
        if (card.Place == "stack")
        {
            return (card.X - card.W / 2 + 6, card.Y);
        }

        int at = state.Cards.ToList().IndexOf(card);
        List<CardState> over = [.. state.Cards.Skip(at + 1).Where(other => other.Visible && other.Place == card.Place && other.Row == card.Row)];

        bool Covered(float x, float y) => over.Any(other =>
            Math.Abs(x - other.X) <= other.W / 2 && Math.Abs(y - other.Y) <= other.H / 2);

        foreach (float down in new[] { 0f, -0.35f, 0.35f, -0.42f, 0.42f })
        {
            foreach (float across in new[] { 0f, -0.3f, 0.3f, -0.42f, 0.42f })
            {
                float x = card.X + (across * card.W);
                float y = card.Y + (down * card.H);
                if (!Covered(x, y))
                {
                    return (x, y);
                }
            }
        }

        return (card.X, card.Y);
    }

    /// <summary>The dock's button at this place in its order, clicked where it is.</summary>
    private async Task ClickDockAsync(int index)
    {
        TableState state = await _page.SettledAsync();
        DockButtonState dock = state.Dock is { } buttons && index < buttons.Count
            ? buttons[index]
            : throw new NotDrivenException($"the dock's button {index}", state);
        // Two buttons may read the same (two effects of permanents nothing tells apart): the one at the same place among them.
        int same = (state.Dock ?? []).Take(index).Count(b => b.Label == dock.Label);
        ControlState button = state.Controls.Where(c => c.Kind == "button" && c.Label == dock.Label).ElementAtOrDefault(same)
            ?? throw new Xunit.Sdk.XunitException($"The dock's button '{dock.Label}' is not a control in view.");
        await _page.TapAsync(button.X, button.Y);
    }

    private async Task ClickButtonNamedAsync(string label)
    {
        TableState state = await _page.SettledAsync();
        ControlState button = state.Controls.FirstOrDefault(c => c.Kind == "button" && c.Label == label)
            ?? throw new Xunit.Sdk.XunitException($"No button '{label}'. There are: {string.Join(", ", state.Controls.Where(c => c.Kind == "button").Select(c => c.Label))}");
        await _page.TapAsync(button.X, button.Y);
    }

    private async Task ClickButtonStartingAsync(string start)
    {
        TableState state = await _page.SettledAsync();
        ControlState button = state.Controls.FirstOrDefault(c => c.Kind == "button" && c.Label.StartsWith(start, StringComparison.Ordinal))
            ?? throw new Xunit.Sdk.XunitException($"No button starting '{start}'. There are: {string.Join(", ", state.Controls.Where(c => c.Kind == "button").Select(c => c.Label))}");
        await _page.TapAsync(button.X, button.Y);
    }

    // --- What the client shows ---

    /// <summary>A <c>Client.</c> line judged against the page; null when it holds, else what was wrong.</summary>
    private async Task<string?> JudgeAsync(ScenarioCheck check, TableState state)
    {
        string[] parts = check.Key.Split('.', 4);
        if (parts.Length < 3 || parts[1] != "Bot1")
        {
            return $"line {check.Line}: {check.Key} is not something the page for Bot1 can say (Client.Bot1.<what>)";
        }

        string what = string.Join('.', parts.Skip(2));

        // What the face of one card says: Client.Bot1.Battlefield.Firdoch Core.TypeLine. A card in exile or a graveyard is
        // only drawn while that zone is open, so the zone is opened to read it, as a player opens it to look.
        Match card = CardKey.Match(what);
        string? shown;
        if (card.Success)
        {
            string row = card.Groups["row"].Value;
            string name = card.Groups["card"].Value;
            string attribute = card.Groups["attr"].Value;
            if (row is "Exile" or "OpponentExile" or "Graveyard")
            {
                shown = await ReadInZoneAsync(row, name, attribute);
            }
            else
            {
                string place = row switch { "OpponentBattlefield" => "opponent-battlefield", _ => row.ToLowerInvariant() };
                // A face-down card the page may look at (a spell cast face down, CR 708.5) is found by what it really is, as the
                // script names it.
                CardState? found = state.Cards.FirstOrDefault(c => c.Place == place && c.Name == name)
                    ?? state.Cards.FirstOrDefault(c => c.Place == place && c.Name.Length == 0 && c.FaceDownName == name);
                shown = found is null
                    ? $"(no {name} in {row}: {string.Join("; ", state.Cards.Where(c => c.Place == place).Select(c => c.Name))})"
                    : Face(found, attribute);
            }
        }
        else
        {
            // A list's .Count, as the dump's lists have one.
            shown = what.EndsWith(".Count", StringComparison.Ordinal) && Read(what[..^".Count".Length], state) is { } list
                ? list.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).Length.ToString(System.Globalization.CultureInfo.InvariantCulture)
                : Read(what, state);

            // The button the last choice of cards was confirmed with.
            if (what == "Confirm")
            {
                shown = _lastConfirm;
            }

            // A line of the game log, by its whole text: the value if the log has it, nothing if not.
            if (what == "Log.Has")
            {
                shown = state.LogText?.Contains(check.Value) == true ? check.Value : string.Empty;
            }
        }

        if (shown is null)
        {
            return $"line {check.Line}: {check.Key} is not a client key: {ClientKeys}";
        }

        return Same(shown, check.Value) ? null : $"line {check.Line}: {check.Key} expected \"{check.Value}\", the page shows \"{shown}\"" +
            (what == "Log.Has" ? "; its log ends: " + string.Join(" | ", (state.LogText ?? []).TakeLast(8)) : string.Empty);
    }

    private static readonly Regex CardKey = new(
        @"^(?<row>Stack|Hand|Battlefield|OpponentBattlefield|Exile|OpponentExile|Graveyard)\.(?<card>.+)\.(?<attr>TypeLine|Rules|Status|Annotation|Counters|Damage|FaceDownName|Halves)$",
        RegexOptions.CultureInvariant);

    /// <summary>One thing a card's face says, with its lines on one line.</summary>
    private static string Face(CardState card, string attribute) => attribute switch
    {
        "TypeLine" => card.TypeLine,
        "Rules" => card.Rules,
        "Status" => card.Status,
        "Counters" => card.Counters,
        "Damage" => card.Damage,
        "FaceDownName" => card.FaceDownName,
        // A split card's halves as its face draws them, side by side: a fused split spell on the stack is both (CR 702.102b).
        "Halves" => card.Halves,
        _ => card.Annotation.ReplaceLineEndings(" / "),
    };

    /// <summary>
    /// What a card's face says in a zone drawn only while it is open: Bot1's exile (the button over the piles) or graveyard
    /// (the pile), opened as a player opens it and shut again after.
    /// </summary>
    private async Task<string> ReadInZoneAsync(string zone, string name, string attribute)
    {
        TableState state = await _page.SettledAsync();
        string place = zone == "Graveyard" ? "graveyard" : "choice";
        (float X, float Y)? opener = ZoneOpener(state, zone);
        if (opener is null)
        {
            return $"(no {zone} to open)";
        }

        await _page.TapAsync(opener.Value.X, opener.Value.Y);
        TableState open = await _page.SettledAsync();
        // A face-down card the page may look at (CR 406.3) is found by what it really is, as the script names it.
        CardState? found = open.Cards.FirstOrDefault(c => c.Place == place && c.Visible
            && (c.Name == (name == "(hidden)" ? string.Empty : name) || (c.Name.Length == 0 && c.FaceDownName == name)));
        string shown = found is null
            ? $"(no {name} in the open {zone}: {string.Join("; ", open.Cards.Where(c => c.Place == place).Select(c => c.Name))})"
            : Face(found, attribute);

        // Shut again, where the zone's opener now is.
        (float X, float Y)? closer = ZoneOpener(open, zone);
        if (closer is not null)
        {
            await _page.TapAsync(closer.Value.X, closer.Value.Y);
            await _page.SettledAsync();
        }

        return shown;
    }

    /// <summary>Where a click opens Bot1's exile or graveyard: the Exile button, or the graveyard pile.</summary>
    private static (float X, float Y)? ZoneOpener(TableState state, string zone)
    {
        PlayerState? me = zone == "OpponentExile"
            ? state.Players?.FirstOrDefault(p => p.Seat != state.Seat)
            : state.Players?.FirstOrDefault(p => p.Seat == state.Seat);
        if (zone is "Exile" or "OpponentExile")
        {
            ZoneButtonState? button = me?.Zones?.FirstOrDefault(z => z.Text.StartsWith("Exile", StringComparison.Ordinal));
            return button is null ? null : (button.X, button.Y);
        }

        return me is { GraveyardX: > 0 } ? (me.GraveyardX, me.GraveyardY) : null;
    }

    private const string ClientKeys =
        "Stack, Hand, Battlefield, OpponentBattlefield, OpponentHand (the cards known in it), each with .Count; <Row>.<Card>.TypeLine, .Rules, .Status, .Annotation, .Counters (the pills drawn on it, like 3 × incubation), .FaceDownName (what a face-down card really is, to the page that may look at it; empty to one that may not) or .Damage (the marked damage drawn beside its power and toughness, like -3), .Halves (a split card's halves as drawn, like Turn; Burn; rows Stack, Hand, Battlefield, OpponentBattlefield, Exile, Graveyard); Facts, OpponentFacts (beside the life total); Mana (the pool's chips, like 2 Blue (land abilities)); Exile.Lit, .Caption, .Tooltip and OpponentExile.*; Sideboard.Caption, .Tooltip, OpponentSideboard.Caption, .Tooltip; Companion, OpponentCompanion (the card in each companion window, empty with none) and their .Corner (bottom-left, top-left); Graveyard.Lit, .Note; Button.<label> (shown or absent), Button.<label>.Enabled (true or false: whether the dock's button can be pressed; absent with none), Arrows (the latest resolution's targets), Over, Prompt.Title, Prompt.Detail, Prompt.Modes, Log.Has (a line of the game log, by its text), Confirm (the button the last choice of cards was confirmed with), Menu (the rows of the menu the last card action opened; empty when it opened none), Targeting (the dock's title / line while the last card action's targets were picked), Dock (the dock's buttons), Combat (each combat group as its ground is captioned, with its attackers: Attacking Bot2: Ornithopter); TargetLabels (the labels on the arrows from the spell on the stack to its targets, like Turn; Burn); Hand.<Card>.Actions, Battlefield.<Card>.Actions (what the card offers to do, as its menu words it)";

    private string? Read(string what, TableState state)
    {
        string Names(string place) => string.Join("; ", state.Cards.Where(c => c.Place == place).Select(c => c.Name));
        PlayerState? me = state.Players?.FirstOrDefault(p => p.Seat == state.Seat);
        PlayerState? them = state.Players?.FirstOrDefault(p => p.Seat != state.Seat);
        static ZoneButtonState? Exile(PlayerState? player) => player?.Zones?.FirstOrDefault(z => z.Text.StartsWith("Exile", StringComparison.Ordinal));
        static ZoneButtonState? Sideboard(PlayerState? player) => player?.Zones?.FirstOrDefault(z => z.Text.StartsWith("Sideboard", StringComparison.Ordinal));
        return what switch
        {
            "Facts" => string.Join("; ", me?.Facts ?? []),
            // The pool's chips as drawn: "2 Blue (land abilities)" for restricted mana, its caption what it is for.
            "Mana" => string.Join("; ", (me?.Mana ?? []).Select(m => m.Caption.Length == 0 ? $"{m.Count} {m.Type}" : $"{m.Count} {m.Type} ({m.Caption})")),
            "OpponentFacts" => string.Join("; ", them?.Facts ?? []),
            "Exile.Lit" => (Exile(me)?.Lit ?? false) ? "true" : "false",
            "Exile.Caption" => Exile(me)?.Text ?? string.Empty,
            "Exile.Tooltip" => Exile(me)?.Tooltip ?? string.Empty,
            "OpponentExile.Lit" => (Exile(them)?.Lit ?? false) ? "true" : "false",
            "OpponentExile.Caption" => Exile(them)?.Text ?? string.Empty,
            "OpponentExile.Tooltip" => Exile(them)?.Tooltip ?? string.Empty,
            // The button into the cards a player owns outside the game (CR 400.11a): on its owner's rail alone.
            "Sideboard.Caption" => Sideboard(me)?.Text ?? string.Empty,
            "Sideboard.Tooltip" => Sideboard(me)?.Tooltip ?? string.Empty,
            "OpponentSideboard.Caption" => Sideboard(them)?.Text ?? string.Empty,
            "OpponentSideboard.Tooltip" => Sideboard(them)?.Tooltip ?? string.Empty,
            // The companion windows (CR 103.2b): the card in each, and the corner of its half it stands in.
            "Companion" => string.Join("; ", state.Cards.Where(c => c.Visible && c.Place == "companion").Select(c => c.Name)),
            "OpponentCompanion" => string.Join("; ", state.Cards.Where(c => c.Visible && c.Place == "opponent-companion").Select(c => c.Name)),
            "Companion.Corner" => state.Companions?.FirstOrDefault(c => c.Whose == "mine")?.Corner ?? string.Empty,
            "OpponentCompanion.Corner" => state.Companions?.FirstOrDefault(c => c.Whose == "opponent")?.Corner ?? string.Empty,
            "Graveyard.Lit" => (me?.GraveyardLit ?? false) ? "true" : "false",
            "Graveyard.Note" => me?.GraveyardNote ?? string.Empty,
            "Stack" => Names("stack"),
            "Hand" => Names("hand"),
            "Battlefield" => Names("battlefield"),
            "OpponentBattlefield" => Names("opponent-battlefield"),
            "OpponentHand" => string.Join("; ", state.Cards.Where(c => c.Place == string.Empty && c.Row == string.Empty && c.Name.Length > 0 && c.Handle == 0).Select(c => c.Name)),
            "Arrows" => state.Effects is { Count: > 0 } effects ? string.Join("; ", effects[^1].Arrows.Select(a => a.To)) : string.Empty,
            "Over" => state.Over ? "true" : "false",
            // A Room's doors as its face shows them: `Battlefield.<Room name>.Doors` is "Unholy Annex:unlocked; Ritual Chamber:locked", and
            // `.Actions` what the card offers to do, as its menu words it. A Room with no door unlocked has no name: `Battlefield..Doors`.
            _ when RoomKey(what, state) is { } shown => shown,
            "Prompt.Title" => state.Title,
            "Prompt.Detail" => state.Detail,
            "Prompt.Modes" => state.Modes is { } modes ? string.Join("; ", modes) : string.Empty,
            "Menu" => _lastMenu,
            "Targeting" => _lastTargeting,
            "Dock" => string.Join("; ", (state.Dock ?? []).Select(b => b.Label)),
            "Combat" => string.Join("; ", state.Combat ?? []),
            "TargetLabels" => string.Join("; ", state.TargetLabels ?? []),
            // `Button.<label>.Enabled`: whether the dock's button whose label holds the text can be pressed ("true" or "false"; "absent" with no
            // such button), as the page publishes it under ?test.
            _ when what.StartsWith("Button.", StringComparison.Ordinal) && what.EndsWith(".Enabled", StringComparison.Ordinal) =>
                (state.Dock ?? []).FirstOrDefault(b => b.Label.Contains(what["Button.".Length..^".Enabled".Length], StringComparison.Ordinal)) is { } dockButton
                    ? (dockButton.Enabled ? "true" : "false")
                    : "absent",
            _ when what.StartsWith("Button.", StringComparison.Ordinal) =>
                state.Controls.Any(c => c.Kind == "button" && c.Label.Contains(what["Button.".Length..], StringComparison.Ordinal)) ? "shown" : "absent",
            _ => null,
        };
    }

    private static string? RoomKey(string what, TableState state)
    {
        foreach ((string prefix, string place) in new[] { ("Battlefield.", "battlefield"), ("OpponentBattlefield.", "opponent-battlefield"), ("Hand.", "hand") })
        {
            foreach (string field in new[] { "Doors", "Actions" })
            {
                string suffix = "." + field;
                if (what.StartsWith(prefix, StringComparison.Ordinal) && what.EndsWith(suffix, StringComparison.Ordinal) && what.Length >= prefix.Length + suffix.Length)
                {
                    string name = what[prefix.Length..^suffix.Length];
                    CardState? card = state.Cards.FirstOrDefault(c => c.Place == place && c.Name == name);
                    return card is null ? null : field == "Doors" ? card.Doors : card.Actions;
                }
            }
        }

        return null;
    }

    /// <summary>Lists compare as sets, as the dump's hand and battlefield do; the value names the empty list by nothing.</summary>
    private static bool Same(string shown, string expected)
    {
        static List<string> Items(string text) => [.. text.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).Order(StringComparer.Ordinal)];
        return Items(shown).SequenceEqual(Items(expected), StringComparer.Ordinal);
    }
}

/// <summary>An answer this driver cannot click its way to yet, naming it and what the page offered.</summary>
public sealed class NotDrivenException(string what, TableState state)
    : Exception($"The browser driver cannot click its way to {what} yet. The page offers: {string.Join(", ", state.Controls.Select(c => $"{c.Kind} '{c.Label}'"))}");
