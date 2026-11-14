using Cantrip.Client.Presentation;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// Every prompt dock the client can show, from the canned positions in <see cref="PromptFixtures"/>: no game is dealt,
/// so there is nothing to reach and nothing to break when a card is added to a deck. One page for them all - the
/// gallery changes fixture by the address's fragment, without reloading - and a picture of each, named after it, under
/// TestResults\web\PromptGalleryTests.
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class PromptGalleryTests(WebClient web)
{
    [Fact]
    public async Task Defined_x_names_the_choice_and_displays_the_spells_definition()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=choose-player-defined-x", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "choose-player-defined-x");
        Assert.Equal("Spoils of War: choose a player to determine X", opened.Title);
        Assert.Contains("artifact and/or creature cards in an opponent's graveyard", opened.Detail, StringComparison.Ordinal);
        await table.ClickAsync("Bot 3", "button");
        TableState answered = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("PlayerChosenAnswer 3", answered.Answered);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Ordered_library_top_names_the_destination_for_graveyard_and_library_cards()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=arrange-opponent-library", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opponent = await table.SettledAsync(state => state.Fixture == "arrange-opponent-library");
        Assert.Equal("Natural Selection: put all 3 cards on top of the library in any order", opponent.Title);
        await table.Page.EvaluateAsync("name => location.hash = name", "arrange-opponent-library-one");
        TableState single = await table.SettledAsync(state => state.Fixture == "arrange-opponent-library-one");
        Assert.Equal("Natural Selection: put the card on top of the library", single.Title);
        await table.Page.EvaluateAsync("name => location.hash = name", "arrange-graveyard-on-library");
        TableState graveyard = await table.SettledAsync(state => state.Fixture == "arrange-graveyard-on-library");
        Assert.Equal("Drafna's Restoration: put all 2 cards on top of the library in any order", graveyard.Title);
        await table.Page.EvaluateAsync("name => location.hash = name", "arrange-library-on-library");
        TableState library = await table.SettledAsync(state => state.Fixture == "arrange-library-on-library");
        Assert.Equal("Ponder: put all 2 cards on top of the library in any order", library.Title);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Pending_delayed_text_is_shown_only_on_the_affected_permanent()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=delayed-carried-object", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState state = await table.SettledAsync(s => s.Fixture == "delayed-carried-object");
        CardState[] walls = [.. state.Cards.Where(c => c.Visible && c.Name == "Steel Wall" && c.Place is "battlefield" or "opponent-battlefield")];
        Assert.Equal(2, walls.Length);
        CardState marked = Assert.Single(walls, c => c.Rules.Contains("Pending delayed ability", StringComparison.Ordinal));
        Assert.Contains("next end step", marked.Rules, StringComparison.Ordinal);
        Assert.Contains("Destroy it", marked.Rules, StringComparison.Ordinal);
        Assert.DoesNotContain("Pending delayed ability", Assert.Single(walls, c => c.Handle != marked.Handle).Rules, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Divided_targets_include_players_and_submit_their_shares()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=divide-targets-players", WebClient.Pictures(nameof(PromptGalleryTests)));
        await table.SettledAsync(state => state.Fixture == "divide-targets-players");
        await table.ClickAsync("More to you (seat 1, now 0)", "button");
        await table.ClickAsync("More to Bot 2 (seat 2, now 0)", "button");
        await table.ClickAsync("More to Bot 2 (seat 2, now 1)", "button");
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Divide" && c.Enabled));
        await table.ClickAsync("Divide", "button");
        TableState answered = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("TargetsDividedAnswer player 1 1; player 2 2", answered.Answered);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Cost_payment_choices_show_the_action_and_send_its_opaque_key()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=cost-payment-order", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "cost-payment-order");
        Assert.Equal("Choose a cost to pay next", opened.Title);
        Assert.Contains(opened.Controls, c => c.Label == "Sacrifice Grizzly Bears for Fling");
        await table.PictureAsync("cost-payment-order");
        await table.ClickAsync("Sacrifice Captain Jean-Luc Picard for casualty", "button");
        TableState chosen = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("CostPaymentChosenAnswer 1", chosen.Answered);
    }

    [Fact]
    public async Task Wide_numbers_are_typed_and_submitted_exactly_with_safe_endpoint_steps()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=number-wide", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "number-wide");
        Assert.Equal("Put counters", opened.Title);
        Assert.Contains("65535", opened.Detail, StringComparison.Ordinal);
        Assert.Contains(opened.Controls, c => c.Label == "−" && !c.Enabled);
        await table.ClickAsync("+", "button");
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Choose 1"));
        await table.ClickAsync("−", "button");
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Choose 0"));
        await table.ClickAsync("Number", "textbox");
        await table.Page.Keyboard.PressAsync("Control+A");
        await table.Page.Keyboard.TypeAsync("65536");
        TableState invalid = await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Choose number"));
        Assert.Contains(invalid.Controls, c => c.Label == "Choose number" && !c.Enabled);
        await table.Page.Keyboard.PressAsync("Backspace");
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Choose 6553" && c.Enabled));
        await table.Page.Keyboard.TypeAsync("5");
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Choose 65535" && c.Enabled));
        await table.PictureAsync("number-wide-65535");
        await table.ClickAsync("Choose 65535", "button");
        TableState chosen = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("NumberChosenAnswer 65535", chosen.Answered);

        await table.Page.EvaluateAsync("name => location.hash = name", "number-maximum");
        await table.SettledAsync(state => state.Fixture == "number-maximum" && state.Answered.Length == 0);
        await table.ClickAsync("Number", "textbox");
        await table.Page.Keyboard.PressAsync("Control+A");
        await table.Page.Keyboard.TypeAsync(uint.MaxValue.ToString());
        TableState maximum = await table.SettledAsync(state => state.Controls.Any(c => c.Label == $"Choose {uint.MaxValue}"));
        Assert.Contains(maximum.Controls, c => c.Kind == "button" && c.Label == "+" && !c.Enabled);
        await table.PictureAsync("number-maximum");
        await table.Page.Keyboard.PressAsync("Enter");
        chosen = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal($"NumberChosenAnswer {uint.MaxValue}", chosen.Answered);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Snow_pool_chip_offers_both_symbol_allocations_and_sends_the_choice()
    {
        await using TablePage table = await TablePage.OpenAsync(web, "prompt=paying-snow", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "paying-snow");
        await table.PictureAsync("snow-pool");
        // The pool chip is a real mouse target; the standard action menu chooses its allocation.
        ManaChipState chip = Assert.Single(opened.Players!.SelectMany(p => p.Mana), mana => mana.Caption == "snow");
        Assert.True(chip.Clickable);
        await table.Page.Mouse.ClickAsync(chip.X, chip.Y);
        await table.SettledAsync(state => state.Controls.Any(c => c.Label == "Pay {S} with snow {U}"));
        await table.PictureAsync("snow-allocation");
        await table.ClickAsync("Pay {S} with snow {U}", "item");
        TableState paid = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Contains("snow=True paySnow=True", paid.Answered, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Searching_an_opponents_library_names_the_opponents_hand()
    {
        await using TablePage table = await TablePage.OpenAsync(
            web, "prompt=search-opponents-hand", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState other = await table.SettledAsync(state => state.Fixture == "search-opponents-hand");
        Assert.Equal("Search Bot 2's library", other.Title);
        Assert.Contains("to put into Bot 2's hand", other.Detail, StringComparison.Ordinal);
        await table.Page.Locator(".uno-loader").WaitForAsync(new()
        {
            State = Microsoft.Playwright.WaitForSelectorState.Hidden,
        });
        await table.PictureAsync("search-opponents-hand");
        await table.Page.EvaluateAsync("name => location.hash = name", "search");
        TableState own = await table.SettledAsync(state => state.Fixture == "search");
        Assert.Equal("Search your library", own.Title);
        Assert.Contains("to put into your hand", own.Detail, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Resolved_regeneration_effects_are_drawn_and_disappear_when_they_end()
    {
        await using TablePage table = await TablePage.OpenAsync(
            web, "prompt=regeneration-effects", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState active = await table.SettledAsync(state => state.Fixture == "regeneration-effects");
        foreach (string name in new[] { "Gurmag Angler", "Savannah Lions" })
        {
            CardState card = Assert.Single(active.Cards, card => card.Visible && card.Name == name
                && card.Place is "battlefield" or "opponent-battlefield");
            Assert.Contains("Regenerate instead of the next destruction this turn.", card.Rules, StringComparison.Ordinal);
            Assert.Contains("Can't be regenerated this combat.", card.Rules, StringComparison.Ordinal);
        }
        await table.Page.Locator(".uno-loader").WaitForAsync(new()
        {
            State = Microsoft.Playwright.WaitForSelectorState.Hidden,
        });
        await table.PictureAsync("regeneration-effects");
        await table.Page.EvaluateAsync("name => location.hash = name", "regeneration-effects-ended");
        TableState ended = await table.SettledAsync(state => state.Fixture == "regeneration-effects-ended");
        foreach (string name in new[] { "Gurmag Angler", "Savannah Lions" })
        {
            CardState card = Assert.Single(ended.Cards, card => card.Visible && card.Name == name
                && card.Place is "battlefield" or "opponent-battlefield");
            Assert.DoesNotContain("Regenerate instead of", card.Rules, StringComparison.Ordinal);
            Assert.DoesNotContain("Can't be regenerated", card.Rules, StringComparison.Ordinal);
        }
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task A_copy_can_keep_its_original_target_without_picking_a_candidate()
    {
        await using TablePage table = await TablePage.OpenAsync(
            web, "prompt=choose-new-target-keep", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "choose-new-target-keep");
        Assert.Contains("Target 1 (Sea Sprite)", opened.Detail, StringComparison.Ordinal);
        Assert.Contains("none to keep the current one", opened.Detail, StringComparison.Ordinal);
        await table.PictureAsync("choose-new-target-keep");
        await table.ClickAsync("Keep current target", "button");
        TableState answered = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.StartsWith("TargetsChosenAnswer", answered.Answered, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Every_prompt_dock_is_drawn()
    {
        IReadOnlyList<string> names = PromptFixtures.Names;
        await using TablePage table = await TablePage.OpenAsync(
            web, $"prompt={Uri.EscapeDataString(names[0])}", WebClient.Pictures(nameof(PromptGalleryTests)));

        foreach (string name in names)
        {
            await table.Page.EvaluateAsync("name => location.hash = name", name);
            // Not every fixture is a question: the lobby is a table before the game, with nothing asked.
            await table.SettledAsync(state => state.Fixture == name);
            await table.PictureAsync(name);
        }

        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Memories_Returning_says_what_the_opponent_chosen_does()
    {
        await using TablePage table = await TablePage.OpenAsync(
            web, "prompt=choose-player-bottom", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "choose-player-bottom");
        Assert.Equal("Memories Returning: choose an opponent", opened.Title);
        Assert.Equal("The player you choose chooses which of the revealed cards go on the bottom of your library, one at a time (up to 2)", opened.Detail);
        await table.ClickAsync("Bot 3", "button");
        TableState chosen = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.StartsWith("PlayerChosenAnswer", chosen.Answered, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    [Fact]
    public async Task Piles_are_separated_with_an_empty_pile_allowed_and_chosen_by_their_contents()
    {
        await using TablePage table = await TablePage.OpenAsync(
            web, "prompt=separate-piles", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == "separate-piles");
        Assert.Equal("Any number of choices for Fact or Fiction", opened.Title);
        Assert.Contains("choose the first pile; the rest form the second", opened.Detail, StringComparison.Ordinal);
        Assert.Contains(opened.Cards, card => card.Visible && card.Name == "Grizzly Bears");
        await table.ClickAsync("Choose none", "button");
        TableState separated = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.StartsWith("TargetsChosenAnswer", separated.Answered, StringComparison.Ordinal);

        await table.Page.EvaluateAsync("name => location.hash = name", "choose-pile");
        TableState choice = await table.SettledAsync(state => state.Fixture == "choose-pile");
        Assert.Contains(choice.Controls, control => control.Label == "First pile: Grizzly Bears, Island");
        Assert.Contains(choice.Controls, control => control.Label == "Second pile: Counterspell, Brainstorm");
        await table.ClickAsync("Second pile: Counterspell, Brainstorm", "button");
        TableState chosen = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("ModesChosenAnswer 1", chosen.Answered);

        await table.Page.EvaluateAsync("name => location.hash = name", "separate-piles");
        await table.SettledAsync(state => state.Fixture == "separate-piles" && state.Answered.Length == 0);
        await table.ClickCardAsync("Grizzly Bears");
        await table.SettledAsync(state => state.Detail.Contains("1 of 4 chosen", StringComparison.Ordinal));
        await table.ClickAsync("Done with 1 choice", "button");
        TableState picked = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.StartsWith("TargetsChosenAnswer", picked.Answered, StringComparison.Ordinal);
        Assert.Empty(table.Errors);
    }

    /// <summary>
    /// Supportive Parents' "Tap two untapped creatures you control: Add one mana of any color.": the colour is named on the
    /// menu row, the dock then asks for the two creatures the cost taps, and the second click activates it with both and the
    /// colour (CR 605.1a - the option's targets are its cost's permanents).
    /// </summary>
    [Fact]
    public async Task A_mana_ability_that_taps_two_creatures_is_activated_with_both_and_its_colour()
    {
        const string Fixture = "priority-tap-two-for-mana";
        await using TablePage table = await TablePage.OpenAsync(
            web, $"prompt={Fixture}", WebClient.Pictures(nameof(PromptGalleryTests)));
        TableState opened = await table.SettledAsync(state => state.Fixture == Fixture);
        int parents = opened.Cards.First(card => card.Name == "Supportive Parents").Handle;
        int bears = opened.Cards.First(card => card.Name == "Grizzly Bears").Handle;

        await table.ClickCardAsync("Supportive Parents");
        await table.ClickAsync("Add {G}");
        TableState asked = await table.SettledAsync(state => state.Title.StartsWith("Tap two untapped creatures", StringComparison.Ordinal));
        Assert.Contains("0 of 2 chosen, from 3", asked.Detail, StringComparison.Ordinal);

        await table.ClickCardAsync("Supportive Parents");
        await table.SettledAsync(state => state.Detail.Contains("1 of 2 chosen", StringComparison.Ordinal));
        Assert.Empty((await table.SettledAsync()).Answered);
        await table.ClickCardAsync("Grizzly Bears");
        TableState activated = await table.SettledAsync(state => state.Answered.Length > 0);

        Assert.Equal($"TakeActionAnswer ActivateAbility {parents} > {parents},{bears} Green", activated.Answered);
        Assert.Empty(table.Errors);
    }

    /// <summary>
    /// A list of options too long for a button each - every creature type - is found by typing: the box narrows the
    /// buttons to what matches, a click on one answers with that option, and Enter takes the one the typing settles on.
    /// </summary>
    [Fact]
    public async Task A_long_option_list_is_found_by_typing()
    {
        const string Fixture = "choose-option-long";
        await using TablePage table = await TablePage.OpenAsync(
            web, $"prompt={Fixture}", WebClient.Pictures(nameof(PromptGalleryTests)));

        TableState opened = await table.SettledAsync(state => state.Fixture == Fixture);
        Assert.Contains(opened.Controls, control => control.Kind == "textbox" && control.Label == "Type to find one");
        Assert.True(Options(opened).Count <= 8, $"Too many option buttons: {string.Join(", ", Options(opened))}");
        Assert.DoesNotContain("Zombie", Options(opened));

        await table.TypeAsync("Type to find one", "dra");
        TableState narrowed = await table.SettledAsync(state => Options(state).Contains("Hydra"));
        Assert.Equal(["Dragon", "Drake", "Eldrazi", "Hydra"], Options(narrowed));
        await table.PictureAsync("typed-dra");

        await table.ClickAsync("Drake", "button");
        TableState clicked = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal($"ModesChosenAnswer {Index("Drake")}", clicked.Answered);

        await table.Page.EvaluateAsync("name => location.hash = name", "modes-one");
        await table.SettledAsync(state => state.Fixture == "modes-one");
        await table.Page.EvaluateAsync("name => location.hash = name", Fixture);
        await table.SettledAsync(state => state.Fixture == Fixture && state.Answered.Length == 0);
        await table.TypeAsync("Type to find one", "time lord");
        await table.SettledAsync(state => Options(state).SequenceEqual(["Time Lord"]));
        await table.Page.Keyboard.PressAsync("Enter");
        TableState entered = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal($"ModesChosenAnswer {Index("Time Lord")}", entered.Answered);

        Assert.Empty(table.Errors);
    }

    /// <summary>
    /// A replacement's source the page holds no handle to - Ral Zarek, gone to the graveyard as its skip was made, seen by a
    /// page reloaded since - is named by the prompt (KI-34), not "An effect".
    /// </summary>
    [Fact]
    public async Task A_replacements_source_the_page_never_saw_is_named_by_the_prompt()
    {
        const string Fixture = "choose-replacement-unseen-source";
        await using TablePage table = await TablePage.OpenAsync(
            web, $"prompt={Fixture}", WebClient.Pictures(nameof(PromptGalleryTests)));

        TableState opened = await table.SettledAsync(state => state.Fixture == Fixture);
        Assert.Equal("Your turn would begin: which applies first?", opened.Title);
        Assert.Equal(
            ["Ral Zarek, Guest Lecturer: you skip this turn", "Time Vault: skip this turn instead of untapping Time Vault"],
            (opened.Dock ?? []).Select(button => button.Label));
        Assert.Empty(table.Errors);
    }

    /// <summary>
    /// Heartless Act's "remove up to three counters from target creature" (CR 608.2d, 122.1): captioned as up to three, its
    /// button removing what is chosen - none at first, then the -1/-1 counters taken - and no more than three can be taken.
    /// </summary>
    [Fact]
    public async Task Up_to_three_counters_are_removed_as_chosen_or_none()
    {
        const string Fixture = "remove-counters-up-to";
        await using TablePage table = await TablePage.OpenAsync(
            web, $"prompt={Fixture}", WebClient.Pictures(nameof(PromptGalleryTests)));

        TableState opened = await table.SettledAsync(state => state.Fixture == Fixture);
        Assert.Equal("Heartless Act: remove up to 3 counters", opened.Title);
        Assert.Contains("0 of up to 3 chosen", opened.Detail, StringComparison.Ordinal);
        Assert.Contains(opened.Controls, control => control.Label == "Remove no counters");

        await table.ClickAsync("Take a -1/-1 counter from Serra Angel (0 of 2)", "button");
        await table.SettledAsync(state => state.Controls.Any(control => control.Label == "Remove 1 counter"));
        await table.ClickAsync("Take a -1/-1 counter from Serra Angel (1 of 2)", "button");
        await table.ClickAsync("Take a +1/+1 counter from Serra Angel (0 of 2)", "button");
        TableState three = await table.SettledAsync(state => state.Controls.Any(control => control.Label == "Remove 3 counters"));
        await table.PictureAsync("remove-counters-up-to-three");

        // Three is the most: a fourth is not taken.
        await table.ClickAsync("Take a +1/+1 counter from Serra Angel (1 of 2)", "button");
        TableState still = await table.SettledAsync();
        Assert.Contains(still.Controls, control => control.Label == "Take a +1/+1 counter from Serra Angel (1 of 2)");
        Assert.Contains("3 of up to 3 chosen", three.Detail, StringComparison.Ordinal);

        await table.ClickAsync("Clear", "button");
        await table.ClickAsync("Take a -1/-1 counter from Serra Angel (0 of 2)", "button");
        await table.ClickAsync("Take a -1/-1 counter from Serra Angel (1 of 2)", "button");
        await table.ClickAsync("Remove 2 counters", "button");
        TableState two = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("CountersRemovedAnswer 9 -1/-1 2", two.Answered);

        await table.Page.EvaluateAsync("name => location.hash = name", "modes-one");
        await table.SettledAsync(state => state.Fixture == "modes-one");
        await table.Page.EvaluateAsync("name => location.hash = name", Fixture);
        await table.SettledAsync(state => state.Fixture == Fixture && state.Answered.Length == 0);
        await table.ClickAsync("Remove no counters", "button");
        TableState none = await table.SettledAsync(state => state.Answered.Length > 0);
        Assert.Equal("CountersRemovedAnswer", none.Answered);

        Assert.Empty(table.Errors);
    }

    /// <summary>The option buttons on the dock: every button but the ones the page always has.</summary>
    private static List<string> Options(TableState state) =>
        [.. state.Controls.Where(control => control.Kind == "button" && Index(control.Label) >= 0).Select(control => control.Label)];

    private static int Index(string option) =>
        PromptFixtures.CreatureTypes.ToList().IndexOf(option);
}
