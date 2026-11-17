using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// Scenarios played in the browser (docs/scenarios-in-the-browser.md): every <c>.cantrip</c> file marked <c># Browser</c>,
/// Bot1 a page clicking its way through the script (<see cref="ScenarioPlayer"/>). <c>CANTRIP_SCENARIO=name</c> plays that
/// one scenario instead, marked or not - with <c>CANTRIP_WEB_HEADED=1</c>, in a browser to watch
/// (<c>tools\scenario-in-browser.cmd</c>).
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class ScenarioInBrowserTests(WebClient web)
{
    /// <summary>The marker that puts a scenario in the browser suite.</summary>
    public const string Marker = "# Browser";

    public static TheoryData<string> Marked()
    {
        var data = new TheoryData<string>();
        string? one = Environment.GetEnvironmentVariable("CANTRIP_SCENARIO");
        if (!string.IsNullOrWhiteSpace(one))
        {
            data.Add(one.Trim().Replace(".cantrip", string.Empty, StringComparison.Ordinal));
            return data;
        }

        foreach (string path in Directory.EnumerateFiles(Path.Combine(WebClient.Root, "scenarios"), "*.cantrip").Order(StringComparer.Ordinal))
        {
            if (File.ReadLines(path).Any(line => line.TrimEnd() == Marker))
            {
                data.Add(Path.GetFileNameWithoutExtension(path));
            }
        }

        return data;
    }

    [Theory]
    [MemberData(nameof(Marked))]
    public async Task A_scenario_plays_through_the_browser(string name)
    {
        string? failure = await ScenarioPlayer.PlayAsync(web, Path.Combine(WebClient.Root, "scenarios", name + ".cantrip"));
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// A stop set on the declare blockers step of one's own turn holds when the opponent declares no blockers (CR 509.4:
    /// the step goes on, and the attacker gets priority in it before damage): with only that stop and the two main phases,
    /// as a player sets them, the page asks Bot1 there and Giant Growth goes on the unblocked attacker.
    /// </summary>
    [Fact]
    public async Task The_attackers_stop_in_the_declare_blockers_step_holds_when_nothing_blocks()
    {
        const string name = "the-attacker-gets-priority-in-the-declare-blockers-step-when-nothing-blocks";
        string? failure = await ScenarioPlayer.PlayAsync(
            web,
            Path.Combine(WebClient.Root, "scenarios", name + ".cantrip"),
            stopsMine: [TurnStepView.PrecombatMain, TurnStepView.DeclareBlockers, TurnStepView.PostcombatMain],
            stopsTheirs: [],
            opponentThinks: TimeSpan.FromMilliseconds(300));
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// Auto-mana with lands of one colour or another: Prismari Charm's {U}{R} paid by clicking Spirebluff Canal and then
    /// Stormcarved Coast, each "{T}: Add {U} or {R}.", with no menu asked - the first for the {U}, the second for the {R}
    /// still owed. The table judges each colour the page chose against the script.
    /// </summary>
    [Fact]
    public async Task Lands_of_one_colour_or_another_are_tapped_by_auto_mana_for_the_colours_still_owed()
    {
        const string name = "prismari-charm-paid-with-two-lands-of-blue-or-red-each-tapped-for-the-colour-still-owed";
        string? failure = await ScenarioPlayer.PlayAsync(web, Path.Combine(WebClient.Root, "scenarios", name + ".cantrip"), autoMana: true);
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// Mana planned for X as it is announced (CR 601.2b, 601.2g): the three Mountains clicked during Fireball's announcement are
    /// not tapped then, raise X to 2, and are tapped by the page once the payment is asked - each tap the answer the script
    /// means, so the table judges the page's own payment as it judges a click.
    /// </summary>
    [Fact]
    public async Task Lands_planned_for_x_raise_it_and_are_tapped_as_the_spell_is_paid_for()
    {
        const string name = "fireball-x-announced-on-the-page-and-paid-with-the-lands-planned-for-it-in-the-browser";
        string? failure = await ScenarioPlayer.PlayAsync(
            web,
            Path.Combine(WebClient.Root, "scenarios", name + ".cantrip"),
            planX: ("Mountain", 3));
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// The defender's side: with the default stops (none on the opponent's turn) a player who was attacked and declared
    /// no blockers is still asked in the declare blockers step, and bolts the unblocked attacker before damage.
    /// </summary>
    [Fact]
    public async Task The_defender_is_asked_in_the_declare_blockers_step_after_declaring_no_blockers()
    {
        const string name = "the-defender-gets-priority-in-the-declare-blockers-step-after-declaring-no-blockers";
        string? failure = await ScenarioPlayer.PlayAsync(
            web,
            Path.Combine(WebClient.Root, "scenarios", name + ".cantrip"),
            stopsMine: [TurnStepView.PrecombatMain, TurnStepView.PostcombatMain],
            stopsTheirs: []);
        Assert.True(failure is null, $"{name}:\n{failure}");
    }
}
