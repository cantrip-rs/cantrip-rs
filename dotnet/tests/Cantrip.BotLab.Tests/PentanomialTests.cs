using Xunit;

namespace Cantrip.BotLab.Tests;

/// <summary>The paired statistics of a duel (<see cref="Pentanomial"/>, <see cref="PairTally"/>).</summary>
public sealed class PentanomialTests
{
    [Fact]
    public void ThePairedShareAndItsIntervalAreTheMeanAndSpreadOfThePairs()
    {
        // Every pair split: a half, and nothing varies.
        Assert.Equal((0.5, 0.0), Pentanomial.Interval([0, 0, 10, 0, 0]));

        // One pair won outright, one lost: mean a half, a pair's variance a quarter, over two pairs.
        (double share, double margin) = Pentanomial.Interval([1, 0, 0, 0, 1]);
        Assert.Equal(0.5, share, 12);
        Assert.Equal(1.96 * Math.Sqrt(0.25 / 2), margin, 12);

        // 10 / 40 / 900 / 60 / 15: mean 520 / 1025 per game; the variance by hand.
        long[] counts = [10, 40, 900, 60, 15];
        double mean = 520.0 / 1025;
        Assert.Equal(mean, Pentanomial.Mean(counts), 12);
        double variance = ((10 * mean * mean) + (40 * Math.Pow(0.25 - mean, 2)) + (900 * Math.Pow(0.5 - mean, 2))
            + (60 * Math.Pow(0.75 - mean, 2)) + (15 * Math.Pow(1 - mean, 2))) / 1025;
        Assert.Equal(variance, Pentanomial.Variance(counts), 12);
        Assert.Equal(1.96 * Math.Sqrt(variance / 1025), Pentanomial.Interval(counts).Margin, 12);

        // Mostly split pairs make the paired variance a small part of the unpaired one.
        Assert.Equal(2 * variance / (mean * (1 - mean)), Pentanomial.VarianceRatio(counts), 12);
        Assert.True(Pentanomial.VarianceRatio(counts) < 0.1);
        Assert.Equal(0, Pentanomial.VarianceRatio([0, 0, 10, 0, 0]));

        // Pairs whose two games are independent coin flips (no draws): 1 / 0 / 2 / 0 / 1, the unpaired variance.
        Assert.Equal(1.0, Pentanomial.VarianceRatio([1, 0, 2, 0, 1]), 12);
    }

    [Fact]
    public void OnTwoOutcomesTheGeneralizedTestIsWalds()
    {
        // Only pairs of 0 and 2 points: a Bernoulli trial a pair, whose distributions of mean s are (1 - s, s), so the
        // GSPRT is Wald's SPRT: n2 ln(s1 / s0) + n0 ln((1 - s1) / (1 - s0)).
        double[] frequencies = [40, 0, 0, 0, 60];
        double wald = (60 * Math.Log(0.55 / 0.5)) + (40 * Math.Log(0.45 / 0.5));
        Assert.Equal(wald, Pentanomial.GeneralizedLlr(frequencies, 0.5, 0.55), 9);
    }

    [Theory]
    // Computed independently (damped Newton steps in the constraints' null space, numpy), each count at least 1e-3.
    [InlineData(new long[] { 10, 40, 900, 60, 15 }, 0.5, 0.52, -3.9423274556098433)]
    [InlineData(new long[] { 120, 300, 2100, 380, 100 }, 0.5, 0.52, -12.24109348558684)]
    [InlineData(new long[] { 0, 3, 2900, 40, 57 }, 0.5, 0.52, 42.714685017266234)]
    [InlineData(new long[] { 5, 20, 50, 20, 5 }, 0.45, 0.55, 0.0)]
    public void ThePentanomialLlrIsTheConstrainedMaximumsRatio(long[] counts, double s0, double s1, double expected) =>
        Assert.Equal(expected, Pentanomial.Llr(counts, s0, s1), 7);

    [Fact]
    public void TheConstrainedMaximumHasTheMeanAndNothingNearItIsMoreLikely()
    {
        double[] pdf = [0.05, 0.1, 0.6, 0.15, 0.1];
        foreach (double s in (double[])[0.3, 0.5, 0.52, 0.7])
        {
            double[] p = Pentanomial.Constrained(pdf, s);
            Assert.Equal(1.0, p.Sum(), 12);
            Assert.Equal(s, p.Select((q, i) => q * Pentanomial.Points[i]).Sum(), 10);

            // Any small move that keeps both constraints (a direction orthogonal to 1 and to x) is less likely.
            double Likelihood(double[] q) => pdf.Select((h, i) => h * Math.Log(q[i])).Sum();
            double best = Likelihood(p);
            double[][] directions = [[1, -2, 1, 0, 0], [0, 1, -2, 1, 0], [0, 0, 1, -2, 1], [1, -1, -1, 1, 0]];
            foreach (double[] d in directions)
            {
                Assert.Equal(0, d.Sum(), 12);
                Assert.Equal(0, d.Select((x, i) => x * Pentanomial.Points[i]).Sum(), 12);
                foreach (double step in (double[])[1e-3, -1e-3])
                {
                    Assert.True(Likelihood([.. p.Select((q, i) => q + (step * d[i]))]) < best);
                }
            }
        }
    }

    [Fact]
    public void TheLlrFavoursTheHypothesisNearerTheData()
    {
        // A clear 60% edge in paired games is H1 at 55% against 50%, and a 45% edge is H0.
        Assert.True(Pentanomial.Llr([10, 20, 100, 60, 30], 0.5, 0.55) > Pentanomial.Bounds(0.05, 0.05).Upper);
        Assert.True(Pentanomial.Llr([30, 60, 100, 20, 10], 0.5, 0.55) < Pentanomial.Bounds(0.05, 0.05).Lower);
        Assert.Equal(0, Pentanomial.Llr([0, 0, 0, 0, 0], 0.5, 0.52), 2);
        Assert.Equal((-Math.Log(19), Math.Log(19)), Pentanomial.Bounds(0.05, 0.05));
    }

    [Fact]
    public void PairsAreCountedOnceBothTheirGamesHaveFinishedInAnyOrder()
    {
        var tally = new PairTally();
        tally.Add(1, 1);
        Assert.Equal(0, tally.Pairs);
        Assert.Equal(1, tally.Incomplete);
        tally.Add(0, 0);
        tally.Add(2, 0.5);
        tally.Add(3, 1);
        tally.Add(5, 1);
        tally.Add(4, 1);
        Assert.Equal([0L, 0, 1, 1, 1], tally.Counts);

        // A pair one of whose games did not finish is never counted, whichever game finished first.
        tally.Fail(6);
        tally.Add(7, 1);
        tally.Add(8, 0);
        tally.Fail(9);
        tally.Fail(10);
        tally.Fail(11);
        tally.Add(12, 1);
        Assert.Equal(3, tally.Pairs);
        Assert.Equal(4, tally.Incomplete);
        Assert.Equal("0 / 0 / 1 / 1 / 1", tally.CountsText);
    }

    [Fact]
    public void TheIntervalStopWaitsForPairsThatDidNotSplit()
    {
        // 96 pairs split and 4 won: 52.0 % +/- 1.9 % at 95 %, wholly above a half, but four decisive pairs say nothing.
        long[] fourWon = [0, 0, 96, 0, 4];
        (double share, double margin) = Pentanomial.Interval(fourWon);
        Assert.True(share - margin > 0.5, "the bare 95% interval would have stopped");
        Assert.Equal(4, Pentanomial.Decisive(fourWon));
        Assert.Equal(0, Pentanomial.IntervalVerdict(fourWon, 1.96, 100, 30));
        Assert.Equal(0, Pentanomial.IntervalVerdict(fourWon, Program.Tally.IntervalZ, Program.Tally.IntervalMinPairs, Program.Tally.IntervalMinDecisive));
        Assert.Equal(2.58, Program.Tally.IntervalZ);
        Assert.Equal(30, Program.Tally.IntervalMinDecisive);

        // Enough decisive pairs, all one way: it stops, either way.
        Assert.Equal(1, Pentanomial.IntervalVerdict([0, 0, 70, 0, 30], 2.58, 100, 30));
        Assert.Equal(-1, Pentanomial.IntervalVerdict([30, 0, 70, 0, 0], 2.58, 100, 30));

        // Too few pairs in all, or an interval that holds a half: no stop.
        Assert.Equal(0, Pentanomial.IntervalVerdict([0, 0, 20, 0, 30], 2.58, 100, 30));
        Assert.Equal(0, Pentanomial.IntervalVerdict([20, 0, 60, 0, 20], 2.58, 100, 30));
    }
}
