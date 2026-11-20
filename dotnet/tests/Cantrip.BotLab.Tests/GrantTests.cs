using System.Globalization;
using Xunit;

namespace Cantrip.BotLab.Tests;

/// <summary>
/// A distributed duel's parts (<see cref="GrantRun"/>) and its summary (<see cref="Program.Summarize"/>): a grant read as
/// self-play reads its own, what a run that died left open closed, and the summary of a duel's results the same lines
/// whatever order and files its rows came in, its sequential stop replayed over the rows.
/// </summary>
public sealed class GrantTests : IDisposable
{
    private readonly string dir = Path.Combine(Path.GetTempPath(), $"cantrip-botlab-grant-{Guid.NewGuid():N}");

    public GrantTests() => Directory.CreateDirectory(dir);

    public void Dispose()
    {
        Directory.Delete(dir, recursive: true);
        Program.Tally.SprtH0 = 0.5;
        Program.Tally.SprtH1 = 0.55;
    }

    [Fact]
    public void AGrantIsReadAsSelfPlayReadsItsOwn()
    {
        (List<int> games, bool ended) = GrantRun.ParseGrant("0..3\n# a comment\n7\n\n10..12\nend\n", 20);
        Assert.Equal([0, 1, 2, 7, 10, 11], games);
        Assert.True(ended);

        // A line not yet ended by its newline grants nothing.
        (games, ended) = GrantRun.ParseGrant("0..2\n4", 20);
        Assert.Equal([0, 1], games);
        Assert.False(ended);

        Assert.Throws<FormatException>(() => GrantRun.ParseGrant("18..21\n", 20));
        Assert.Throws<FormatException>(() => GrantRun.ParseGrant("x\n", 20));
    }

    [Fact]
    public void WhatARunThatDiedLeftOpenIsClosedToItsLastWholeLine()
    {
        File.WriteAllText(Path.Combine(dir, "r1-0000.results.tsv.part"),
            "# {\"h\":1}\n" + GrantRun.Columns + "\n" + Row(0, 1, 1) + "\n" + Row(1, 2, 2) + "\n2\t1\t10");
        Directory.CreateDirectory(Path.Combine(dir, "r2.replay.part", "record"));

        Assert.Equal(2, GrantRun.Recover(dir));

        string[] closed = Directory.GetFiles(dir);
        Assert.Equal([Path.Combine(dir, "r1-0000.recovered.results.tsv")], closed);
        Assert.Equal("# {\"h\":1}\n" + GrantRun.Columns + "\n" + Row(0, 1, 1) + "\n" + Row(1, 2, 2) + "\n", File.ReadAllText(closed[0]));
        Assert.Empty(Directory.GetDirectories(dir));
    }

    /// <summary>A result line: game, its pair, A's seat (by the game), the winner, A's points.</summary>
    private static string Row(int game, int winner, int starter, string deck1 = "ux", string deck2 = "gx")
    {
        int seatOfA = game % 2 == 1 ? 2 : 1;
        string points = winner == 0 ? "0.5" : winner == seatOfA ? "1" : "0";
        return string.Join('\t', game.ToString(CultureInfo.InvariantCulture), (game / 2).ToString(CultureInfo.InvariantCulture), "1000",
            seatOfA.ToString(CultureInfo.InvariantCulture), deck1, deck2, winner.ToString(CultureInfo.InvariantCulture), points,
            starter.ToString(CultureInfo.InvariantCulture), (10 + game).ToString(CultureInfo.InvariantCulture));
    }

    private static string Failed(int game) =>
        string.Join('\t', game.ToString(CultureInfo.InvariantCulture), (game / 2).ToString(CultureInfo.InvariantCulture), "1000",
            (game % 2 == 1 ? 2 : 1).ToString(CultureInfo.InvariantCulture), "ux", "gx", "-", "-", "1", "4");

    private string Summary(IEnumerable<string> rows, Program.Stops stops = default, string header = "{\"h\":1}")
    {
        string path = Path.Combine(dir, $"{Guid.NewGuid():N}.tsv");
        File.WriteAllText(path, GrantRun.Text(header, rows));
        TextWriter before = Console.Out;
        var text = new StringWriter();
        Console.SetOut(text);
        try
        {
            int code = Program.Summarize(path, "a", "b", "{\"h\":1}", stops, 12);
            return $"{code}\n{text}";
        }
        finally
        {
            Console.SetOut(before);
        }
    }

    [Fact]
    public void TheSummaryOfADuelIsTheSameInAnyOrderOfItsGames()
    {
        // 40 games: A wins the even games of the first pairs, draws one, splits the rest.
        List<string> rows = [];
        for (int game = 0; game < 40; game++)
        {
            int winner = game < 12 ? (game % 2 == 0 ? 1 : 2) : game == 13 ? 0 : (game % 3 == 0 ? 1 : 2);
            rows.Add(Row(game, winner, game % 2 + 1, game % 4 < 2 ? "ux" : "rx", "gx"));
        }

        string whole = Summary(rows);
        Assert.StartsWith("0\n", whole, StringComparison.Ordinal);
        Assert.Contains("a beat b ", whole, StringComparison.Ordinal);
        Assert.Contains("Pairs by a's points in a pair (0 / 0.5 / 1 / 1.5 / 2): ", whole, StringComparison.Ordinal);
        Assert.DoesNotContain("An answer took", whole, StringComparison.Ordinal);

        // The hosts' parts, in another order: the same lines (the counts are a sum).
        List<string> split = [.. rows.Where((_, k) => k % 3 == 1), .. rows.Where((_, k) => k % 3 != 1).Reverse()];
        Assert.Equal(whole, Summary(split));
    }

    [Fact]
    public void AGameThatDidNotFinishFailsTheSummaryAsItFailsTheDuel()
    {
        string summary = Summary([Row(0, 1, 1), Row(1, 1, 1), Failed(2), Row(3, 2, 2)]);
        Assert.StartsWith("1\n", summary, StringComparison.Ordinal);
        Assert.Contains("1 game(s) did not finish", summary, StringComparison.Ordinal);
        Assert.Contains("1 pairs, 1 incomplete", summary, StringComparison.Ordinal);
    }

    [Fact]
    public void AnotherDuelsResultsAreRefused()
    {
        Assert.StartsWith("2\n", Summary([Row(0, 1, 1)], header: "{\"h\":2}"), StringComparison.Ordinal);
    }

    [Fact]
    public void ASequentialStopIsReplayedOverTheRowsInTheirOrder()
    {
        Program.Tally.SprtH0 = 0.5;
        Program.Tally.SprtH1 = 0.55;

        // A wins every game: the SPRT accepts H1 after the games its LLR needs, and the games after are counted.
        List<string> rows = [.. Enumerable.Range(0, 80).Select(game => Row(game, game % 2 == 1 ? 2 : 1, 1))];
        string summary = Summary(rows, new Program.Stops(Sprt: true, PairedSprt: false, Interval: false));
        Assert.Contains("SPRT verdict after 31 games", summary, StringComparison.Ordinal);
        Assert.Contains("H1 accepted, a is the better at game 31", summary, StringComparison.Ordinal);
        Assert.Contains("a beat b 80 to 0", summary, StringComparison.Ordinal);
    }
}
