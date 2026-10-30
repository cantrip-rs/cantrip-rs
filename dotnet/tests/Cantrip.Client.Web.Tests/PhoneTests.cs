using Xunit;

namespace Cantrip.Client.Web.Tests;

/// <summary>
/// A player's phone at an in-person table (docs/in-person.md, docs/web-client.md "In-person play"): the ordinary page opened
/// with <c>?inperson=phone</c>, upright at a phone's size on a touch screen, with the battlefield on the tablets. Scenarios
/// played with Bot1 that phone (<see cref="ScenarioPlayer"/> with <c>phone</c>): the hand and the dock tapped with a finger,
/// and every permanent and player tapped on a tablet instead, handed to the phone as the relay hands it a tap
/// (<c>globalThis.cantripTableTaps</c>, the page's test-only stand-in for the relay). The table judges each answer against
/// the script, so a tap on a tablet is shown to answer exactly what a click on the phone would. Pictured at every question,
/// under TestResults\web\ScenarioPlayer\&lt;scenario&gt;-phone.
/// </summary>
[Collection(WebClientCollection.Name)]
public sealed class PhoneTests(WebClient web)
{
    private static string Scenario(string name) => Path.Combine(WebClient.Root, "scenarios", name + ".cantrip");

    /// <summary>
    /// Lightning Bolt at Tarmogoyf: the Mountain tapped on the tablet for {R} at priority, Bolt tapped in the phone's hand,
    /// the Goyf tapped on the other tablet as its target. Before the Mountain, three taps the phone must not use: a player
    /// who is no choice, a tap for a request already past (let go at once) and one for a request still to come (held about a
    /// second for its prompt, then let go) - each said on the phone, none moving the question on.
    /// </summary>
    [Fact]
    public async Task Bolt_is_aimed_at_the_goyf_with_the_mountain_tapped_on_the_table_at_priority()
    {
        const string name = "lightning-bolt-kills-tarmogoyf-with-the-mountain-tapped-first";
        string? failure = await ScenarioPlayer.PlayAsync(web, Scenario(name), phone: true, probeRefusals: true);
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// Fireball at a player: Bot2 tapped on the tablet as the target, and each Mountain tapped on the tablet for a mana
    /// ability while the spell is paid for.
    /// </summary>
    [Fact]
    public async Task Fireball_is_aimed_at_a_player_and_paid_for_with_lands_tapped_on_the_table()
    {
        const string name = "fireball-x-announced-on-the-page-and-paid-with-the-lands-planned-for-it-in-the-browser";
        string? failure = await ScenarioPlayer.PlayAsync(web, Scenario(name), phone: true);
        Assert.True(failure is null, $"{name}:\n{failure}");
    }

    /// <summary>
    /// Engineered Explosives' X raised by the Mountain tapped on the tablet while X is announced (CR 601.2b): planned for X,
    /// not tapped then, and tapped by the phone itself once the payment is asked (CR 601.2g), as a Mountain clicked on the
    /// phone is. (One source, since several planned for X meet KI-134 on any page.)
    /// </summary>
    [Fact]
    public async Task Explosives_x_is_planned_with_the_mountain_tapped_on_the_table_as_it_is_announced()
    {
        const string name = "engineered-explosives-x-raised-by-a-mountain-planned-as-it-is-announced";
        string? failure = await ScenarioPlayer.PlayAsync(web, Scenario(name), planX: ("Mountain", 1), phone: true);
        Assert.True(failure is null, $"{name}:\n{failure}");
    }
}
