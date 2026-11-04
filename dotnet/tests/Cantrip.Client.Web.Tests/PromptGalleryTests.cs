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
