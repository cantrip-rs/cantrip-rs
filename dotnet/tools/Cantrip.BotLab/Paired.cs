using System.Globalization;

namespace Cantrip.BotLab;

/// <summary>
/// The statistics of games played in pairs: the pentanomial model chess engine testing uses for paired games.
/// <para>
/// A duel plays its games in pairs, games 2k and 2k + 1, dealt from one shuffle with the players exchanged: in a true
/// duplicate pair both games have the same deal, the same decks in the same seats, and only who sits where differs.
/// What the deal gives one seat it gives each player once, so it cancels within the pair, and the pair, not the game,
/// is the independent trial. A's score in a pair is 0, 0.5, 1, 1.5 or 2 (a draw a half), and the five counts of pairs
/// are the pentanomial. Two players that play alike split every pair 1-1, and a pair that splits says nothing about
/// which is the better: the information is in the pairs that do not split, so the paired interval is narrower than the
/// unpaired one by as much as the pairs split.
/// </para>
/// <para>
/// Everything here is on the per-game scale: a pair's score is halved (0, 0.25, 0.5, 0.75, 1), so its mean is A's
/// expected share of a game, comparable to the unpaired share.
/// </para>
/// </summary>
internal static class Pentanomial
{
    /// <summary>A pair's score halved, by its index among the five counts: 0, 0.5, 1, 1.5 and 2 points.</summary>
    public static readonly double[] Points = [0, 0.25, 0.5, 0.75, 1];

    /// <summary>
    /// The least a count is taken as in the likelihood ratio, so that the constrained maximum always exists (a
    /// pentanomial with an empty cell may have no distribution of the hypothesised mean on its support). Fishtest's
    /// LLR regularizes its frequencies the same way.
    /// </summary>
    public const double Epsilon = 1e-3;

    /// <summary>The pairs counted.</summary>
    public static long Pairs(IReadOnlyList<long> counts) => counts.Sum();

    /// <summary>A's mean score per game, by pair: the paired share.</summary>
    public static double Mean(IReadOnlyList<long> counts)
    {
        long n = Pairs(counts);
        return n == 0 ? 0 : counts.Select((c, i) => c * Points[i]).Sum() / n;
    }

    /// <summary>The variance of one pair's halved score about the mean (the empirical distribution's own, over N).</summary>
    public static double Variance(IReadOnlyList<long> counts)
    {
        long n = Pairs(counts);
        if (n == 0)
        {
            return 0;
        }

        double mean = Mean(counts);
        return counts.Select((c, i) => c * (Points[i] - mean) * (Points[i] - mean)).Sum() / n;
    }

    /// <summary>
    /// The paired share and its interval's half-width, <c>z sqrt(var / N)</c> over N pairs: the normal approximation to
    /// the mean of independent pairs (1.96 for 95%).
    /// </summary>
    public static (double Share, double Margin) Interval(IReadOnlyList<long> counts, double z = 1.96)
    {
        long n = Pairs(counts);
        return n == 0 ? (0, 0) : (Mean(counts), z * Math.Sqrt(Variance(counts) / n));
    }

    /// <summary>
    /// The paired variance against the unpaired one for the same games: <c>2 var / (s (1 - s))</c>, where an unpaired
    /// game's variance is s (1 - s) (draws aside) and a pair holds two games. Below 1 the pairing narrows the interval
    /// (the same interval in that share of the games); zero when nothing varies.
    /// </summary>
    public static double VarianceRatio(IReadOnlyList<long> counts)
    {
        double s = Mean(counts);
        double unpaired = s * (1 - s);
        return unpaired <= 0 ? 0 : 2 * Variance(counts) / unpaired;
    }

    /// <summary>
    /// The generalized sequential probability ratio test's log-likelihood ratio for the pentanomial, H1 (A's expected
    /// share of a game is <paramref name="s1"/>) against H0 (<paramref name="s0"/>): N times the mean log of the
    /// ratio of the two maximum-likelihood distributions on the five outcomes whose means are s1 and s0
    /// (<see cref="Constrained"/>). This is fishtest's GSPRT for paired games (M. Van den Bergh, "Generalized SPRT"
    /// and "Support of the MLE of a multinomial with a constrained mean", the LLR of its <c>LLRcalc.LLR</c>), with the
    /// hypotheses given as expected scores. Each count is taken as at least <see cref="Epsilon"/>.
    /// </summary>
    public static double Llr(IReadOnlyList<long> counts, double s0, double s1)
    {
        double[] regularized = [.. counts.Select(c => Math.Max(c, Epsilon))];
        return GeneralizedLlr(regularized, s0, s1);
    }

    /// <summary>
    /// The GSPRT's LLR for frequencies (any non-negative counts, at least one positive below and above each mean): N
    /// sum p_i ln(p1_i / p0_i), p the empirical distribution and p0, p1 the constrained maxima. A cell with no count
    /// adds nothing.
    /// </summary>
    internal static double GeneralizedLlr(IReadOnlyList<double> frequencies, double s0, double s1)
    {
        double n = frequencies.Sum();
        if (n <= 0)
        {
            return 0;
        }

        double[] pdf = [.. frequencies.Select(f => f / n)];
        double[] p0 = Constrained(pdf, s0);
        double[] p1 = Constrained(pdf, s1);
        double sum = 0;
        for (int i = 0; i < pdf.Length; i++)
        {
            if (pdf[i] > 0)
            {
                sum += pdf[i] * Math.Log(p1[i] / p0[i]);
            }
        }

        return n * sum;
    }

    /// <summary>
    /// The distribution on <see cref="Points"/> of mean <paramref name="s"/> that is most likely to have produced the
    /// empirical distribution <paramref name="pdf"/>: maximizing sum pdf_i ln p_i under sum p_i = 1 and
    /// sum p_i (x_i - s) = 0 gives p_i = pdf_i / (1 + theta (x_i - s)) by Lagrange, with theta the root of
    /// sum pdf_i (x_i - s) / (1 + theta (x_i - s)) = 0 (decreasing in theta) between -1 / max(x_i - s) and
    /// -1 / min(x_i - s), the cells with mass taken. Found by bisection. When s is outside the support's range there
    /// is no such distribution, and the empirical one's nearest end is returned (callers regularize first).
    /// </summary>
    internal static double[] Constrained(IReadOnlyList<double> pdf, double s)
    {
        double[] a = [.. Points.Select(x => x - s)];
        double high = double.NegativeInfinity;
        double low = double.PositiveInfinity;
        for (int i = 0; i < pdf.Count; i++)
        {
            if (pdf[i] > 0)
            {
                high = Math.Max(high, a[i]);
                low = Math.Min(low, a[i]);
            }
        }

        if (!(low < 0 && high > 0))
        {
            return [.. pdf];
        }

        double Slope(double theta)
        {
            double f = 0;
            for (int i = 0; i < pdf.Count; i++)
            {
                if (pdf[i] > 0)
                {
                    f += pdf[i] * a[i] / (1 + (theta * a[i]));
                }
            }

            return f;
        }

        double lo = -1 / high;
        double hi = -1 / low;
        for (int step = 0; step < 200; step++)
        {
            double mid = (lo + hi) / 2;
            if (mid <= lo || mid >= hi)
            {
                break;
            }

            if (Slope(mid) > 0)
            {
                lo = mid;
            }
            else
            {
                hi = mid;
            }
        }

        double root = (lo + hi) / 2;
        double[] p = [.. pdf.Select((q, i) => q > 0 ? q / (1 + (root * a[i])) : 0)];
        double total = p.Sum();
        return [.. p.Select(x => x / total)];
    }

    /// <summary>Wald's bounds for the LLR: accept H0 at or below ln(beta / (1 - alpha)), H1 at or above ln((1 - beta) / alpha).</summary>
    public static (double Lower, double Upper) Bounds(double alpha, double beta) =>
        (Math.Log(beta / (1 - alpha)), Math.Log((1 - beta) / alpha));

    /// <summary>The pairs that did not split 1-1: those that say which player is the better (all but the middle count).</summary>
    public static long Decisive(IReadOnlyList<long> counts) => Pairs(counts) - counts[2];

    /// <summary>
    /// The interval stop's verdict on <paramref name="counts"/>: +1 when the paired <paramref name="z"/>-sigma interval
    /// is wholly above a half, -1 wholly below, 0 while it holds a half, while fewer than <paramref name="minPairs"/>
    /// pairs are counted, or while fewer than <paramref name="minDecisive"/> of them did not split. The interval's
    /// normal approximation needs the decisive pairs, not the split ones, which only narrow it: 96 split pairs and 4
    /// won ones are 52.0 % +/- 1.9 % at 95 %, and a stop on them would promote on four pairs. The duel looks after every
    /// pair, which inflates a fixed-sample interval's error, so the default z is 2.58 (99 %).
    /// </summary>
    public static int IntervalVerdict(IReadOnlyList<long> counts, double z, long minPairs, long minDecisive)
    {
        if (Pairs(counts) < Math.Max(minPairs, 1) || Decisive(counts) < Math.Max(minDecisive, 1))
        {
            return 0;
        }

        (double share, double margin) = Interval(counts, z);
        return share - margin > 0.5 ? 1 : share + margin < 0.5 ? -1 : 0;
    }
}

/// <summary>
/// A duel's games gathered into their pairs (games 2k and 2k + 1) as they finish, in any order: a pair is counted once
/// both its games have, by A's points in it. A pair with a game that did not finish is never counted.
/// </summary>
internal sealed class PairTally
{
    private readonly Dictionary<int, double> waiting = [];

    /// <summary>The pairs by A's points in them: 0, 0.5, 1, 1.5, 2.</summary>
    public long[] Counts { get; } = new long[5];

    /// <summary>Pairs one of whose games did not finish, so never counted.</summary>
    public int Broken { get; private set; }

    /// <summary>
    /// Pairs not counted: one of their games did not finish, or one has finished and the other never did (a duel
    /// stopped early leaves some).
    /// </summary>
    public int Incomplete => Broken + waiting.Count;

    public long Pairs => Counts.Sum();

    /// <summary>A's score in game <paramref name="game"/>: 1 a win, 0 a loss, a half a draw.</summary>
    public void Add(int game, double score)
    {
        int pair = game / 2;
        if (waiting.Remove(pair, out double other))
        {
            if (double.IsNaN(other))
            {
                Broken++;
                return;
            }

            Counts[(int)Math.Round((score + other) * 2)]++;
        }
        else
        {
            waiting[pair] = score;
        }
    }

    /// <summary>Game <paramref name="game"/> did not finish: its pair is never counted.</summary>
    public void Fail(int game)
    {
        int pair = game / 2;
        if (waiting.Remove(pair, out _))
        {
            Broken++;
        }
        else
        {
            waiting[pair] = double.NaN;
        }
    }

    /// <summary>The paired share and its 95% half-width.</summary>
    public (double Share, double Margin) Interval(double z = 1.96) => Pentanomial.Interval(Counts, z);

    /// <summary>The five counts as the duel prints them, 0 first: <c>12 / 30 / 2900 / 40 / 18</c>.</summary>
    public string CountsText => string.Join(" / ", Counts.Select(c => c.ToString(CultureInfo.InvariantCulture)));
}
