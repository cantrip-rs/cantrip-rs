using Cantrip.Protocol;
using Xunit;

namespace Cantrip.Client.Tests;

/// <summary>
/// The announcement as the page steps it (CR 601.2b): the total at each value, worked out from the one the question was
/// asked at in the server's own notation, and the X that the mana planned for it reaches.
/// </summary>
public sealed class AnnouncementTests
{
    private static PayCostsPromptMessage Asked(string total, int x = 0, int most = 1000, int least = 0) =>
        new() { Total = total, Remaining = total, HasX = true, X = x, AnnouncesXOrKicks = true, MostAnnounced = most, LeastAnnounced = least };

    [Theory]
    [InlineData("{X}{R}", 0, 3, "{X}{3}{R}")]
    [InlineData("{X}{3}{R}", 3, 0, "{X}{R}")]
    [InlineData("{X}{X}{G}", 0, 2, "{X}{X}{4}{G}")]
    [InlineData("{X}", 0, 0, "{X}")]
    [InlineData("{X}{W/U}{2/B}{G/P}{W}{U}", 0, 1, "{X}{1}{W/U}{2/B}{G/P}{W}{U}")]
    public void An_x_adds_its_generic_mana_for_each_one(string total, int askedAt, int value, string expected) =>
        Assert.Equal(expected, Announcement.TotalAt(Asked(total, askedAt), value));

    [Fact]
    public void A_multikicker_adds_its_cost_for_each_kick_in_the_servers_order()
    {
        PayCostsPromptMessage kicking = Asked("{2}{B}");
        kicking.Kicker = "{1}{G}";
        kicking.Multikicker = true;
        Assert.Equal("{4}{B}{G}{G}", Announcement.TotalAt(kicking, 2));
    }

    [Fact]
    public void A_kicker_with_x_adds_its_other_symbols_once_and_its_x_for_each()
    {
        PayCostsPromptMessage kicking = Asked("{2}{B}");
        kicking.Kicker = "{X}{2}";
        kicking.VariableKicker = true;
        Assert.Equal("{2}{B}", Announcement.TotalAt(kicking, 0));
        Assert.Equal("{7}{B}", Announcement.TotalAt(kicking, 3));
        Assert.True(Announcement.KickerPaid(kicking, 3));
        Assert.False(Announcement.KickerPaid(kicking, 0));
    }

    [Fact]
    public void A_reduction_for_each_sacrificed_stops_at_no_generic_mana()
    {
        PayCostsPromptMessage sacrificing = Asked("{4}{B}{B}");
        sacrificing.SacrificeReduces = 1;
        Assert.Equal("{2}{B}{B}", Announcement.TotalAt(sacrificing, 2));
        Assert.Equal("{B}{B}", Announcement.TotalAt(sacrificing, 6));
    }

    [Fact]
    public void An_x_paid_in_life_leaves_the_mana_as_it_is()
    {
        PayCostsPromptMessage life = Asked("{2}{B}");
        life.XPaidInLife = true;
        Assert.Equal("{2}{B}", Announcement.TotalAt(life, 5));
    }

    [Theory]
    [InlineData("{X}{R}", 1, 0)]
    [InlineData("{X}{R}", 4, 3)]
    [InlineData("{X}{X}{R}", 6, 2)]
    [InlineData("{X}{3}{R}", 6, 5)]
    [InlineData("{X}{R}", 0, 0)]
    public void Mana_planned_pays_the_rest_of_the_cost_first_and_x_with_what_is_left(string total, int mana, int x) =>
        Assert.Equal(x, Announcement.XPaidBy(Asked(total, total.Contains("{3}", StringComparison.Ordinal) ? 3 : 0), mana));

    [Fact]
    public void The_x_mana_reaches_is_never_past_what_may_be_announced() =>
        Assert.Equal(2, Announcement.XPaidBy(Asked("{X}{R}", most: 2), 10));

    [Fact]
    public void A_value_is_kept_within_what_may_be_announced()
    {
        Assert.Equal(0, Announcement.Clamp(Asked("{X}{R}", most: 2), -1));
        Assert.Equal(2, Announcement.Clamp(Asked("{X}{R}", most: 2), 3));
    }

    [Fact]
    public void An_x_that_cannot_be_0_is_kept_at_1_or_more()
    {
        Assert.Equal(1, Announcement.Clamp(Asked("{X}{G}", x: 1, least: 1), 0));
        Assert.Equal(1, Announcement.XPaidBy(Asked("{X}{G}", x: 1, least: 1), 1));
        Assert.Equal(3, Announcement.Clamp(Asked("{X}{G}", x: 1, least: 1), 3));
    }

    [Theory]
    [InlineData("{X}{2}{R}", 3)]
    [InlineData("{2/W}{B/P}", 3)]
    [InlineData("{0}", 0)]
    public void A_cost_comes_to_its_mana_value_with_x_as_nothing(string cost, int value) =>
        Assert.Equal(value, Announcement.ManaValue(cost));

    /// <summary>Squad (CR 702.157a) is not kicker, and what limits it is cards in the graveyard, not mana.</summary>
    [Fact]
    public void Squad_is_captioned_as_squad_and_limited_by_its_cards_not_by_mana()
    {
        PayCostsPromptMessage squad = Asked("{2}{B}", most: 2);
        squad.Kicker = "Exile four other cards from your graveyard";
        squad.Multikicker = true;
        squad.Squad = true;

        Assert.Equal("Pay squad for Ruthless Radrat?", Announcement.Title(squad, "Ruthless Radrat"));
        Assert.Equal(
            "squad can be paid up to 2 times (Exile four other cards from your graveyard each)",
            Announcement.SquadReach(squad));
        Assert.Equal("Pay squad once: Exile four other cards from your graveyard", Announcement.SquadRaiseHint(squad));
        Assert.Equal("squad paid 2 times", Announcement.SquadPaid(2));
        Assert.Equal("squad not paid", Announcement.SquadPaid(0));

        string[] words = [Announcement.Title(squad, "Ruthless Radrat"), Announcement.SquadReach(squad), Announcement.SquadRaiseHint(squad), Announcement.SquadPaid(2)];
        Assert.All(words, text =>
        {
            Assert.DoesNotContain("kick", text, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("mana", text, StringComparison.OrdinalIgnoreCase);
        });

        squad.MostAnnounced = 0;
        Assert.Equal("squad cannot be paid", Announcement.SquadReach(squad));

        PayCostsPromptMessage kicker = Asked("{2}{B}");
        kicker.Kicker = "{1}{G}";
        Assert.Equal("Kick Spell?", Announcement.Title(kicker, "Spell"));
    }
}
