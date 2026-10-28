using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>
/// The announcement of X, or of how many times a spell is kicked (CR 601.2b), as a page changes it before announcing it: the
/// value is the page's own until it is sent as one <see cref="AnnounceAnswer"/>, so what the total comes to at each value is
/// worked out here from the one the prompt was asked at, in the server's notation (<c>{X}{3}{R}</c>).
/// </summary>
public static class Announcement
{
    /// <summary>
    /// The value the page starts from and steps within: the prompt's own, never under nor past what may be announced (an X
    /// that can't be 0 starts at 1).
    /// </summary>
    public static int Clamp(PayCostsPromptMessage prompt, int value)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        int least = Math.Max(0, prompt.LeastAnnounced);
        return Math.Clamp(value, least, Math.Max(least, prompt.MostAnnounced));
    }

    /// <summary>
    /// The question an announcement asks, truthfully for what the optional additional cost is: squad (CR 702.157a) is not a
    /// kicker, and a buyback is not a kick.
    /// </summary>
    public static string Title(PayCostsPromptMessage prompt, string spell)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        return prompt.Kicker.Length == 0 ? $"Choose X for {spell}"
            : prompt.Squad ? $"Pay squad for {spell}?"
            : prompt.Buyback ? $"Buy back {spell}?"
            : $"Kick {spell}?";
    }

    /// <summary>
    /// What raising the squad count does, for the button that raises it: "Pay squad once more: exile four other cards from
    /// your graveyard".
    /// </summary>
    public static string SquadRaiseHint(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        return $"Pay squad {(prompt.X == 0 ? "once" : "once more")}: {prompt.Kicker}";
    }

    /// <summary>
    /// How far squad can go, in words, from the limit the rules and the cards give it
    /// (<see cref="PayCostsPromptMessage.MostAnnounced"/>): the cards its cost exiles, never mana.
    /// </summary>
    public static string SquadReach(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        return prompt.MostAnnounced switch
        {
            <= 0 => "squad cannot be paid",
            1 => $"squad can be paid once ({prompt.Kicker})",
            var times => $"squad can be paid up to {times} times ({prompt.Kicker} each)",
        };
    }

    /// <summary>What the announced value says of squad once chosen: "squad paid 2 times".</summary>
    public static string SquadPaid(int value) => value switch
    {
        <= 0 => "squad not paid",
        1 => "squad paid",
        var times => $"squad paid {times} times",
    };

    /// <summary>Whether the value announced is a kicker paid (CR 702.33): a kicker with {X} in it is paid for any X but none.</summary>
    public static bool KickerPaid(PayCostsPromptMessage prompt, int value)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        return prompt.Kicker.Length > 0 && value > 0;
    }

    /// <summary>The mana a cost in the server's notation comes to, an {X} counting nothing: <c>{X}{2}{R}</c> is 3.</summary>
    public static int ManaValue(string cost) => Cost.Parse(cost ?? string.Empty).ManaValue;

    /// <summary>How many {X} the cost has: each one more X adds that much generic mana.</summary>
    public static int XSymbols(string cost) => Symbols(cost).Count(symbol => symbol == "X");

    /// <summary>
    /// The total cost with <paramref name="value"/> announced: the prompt's total at its own X, with X's generic mana, the
    /// kicker's cost, or a sacrifice's reduction added or taken off for each step between them.
    /// </summary>
    public static string TotalAt(PayCostsPromptMessage prompt, int value)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        int steps = value - prompt.X;
        if (steps == 0 || prompt.XPaidInLife || prompt.XRemovesLoyalty)
        {
            return prompt.Total;
        }

        Cost cost = Cost.Parse(prompt.Total);

        // "Costs {1} less for each permanent sacrificed this way": a reduction never takes the generic mana below none
        // (CR 118.7a), so it is worked out from the cost with none sacrificed.
        if (prompt.SacrificeReduces > 0)
        {
            cost.Generic = Math.Max(0, cost.Generic + (prompt.X * prompt.SacrificeReduces) - (value * prompt.SacrificeReduces));
            return cost.ToString();
        }

        if (prompt.Kicker.Length == 0)
        {
            cost.Generic = Math.Max(0, cost.Generic + (steps * XSymbols(prompt.Total)));
            return cost.ToString();
        }

        Cost kicker = Cost.Parse(prompt.Kicker);

        // A kicker with {X} in it (CR 702.33, 107.3) is paid once, for the X announced: its other symbols once, and its X's
        // generic mana that many times.
        if (prompt.VariableKicker)
        {
            int perX = kicker.X;
            kicker.X = 0;
            if (prompt.KickerPaid)
            {
                cost.Take(kicker);
                cost.Generic = Math.Max(0, cost.Generic - (prompt.X * perX));
            }

            if (value > 0)
            {
                cost.Add(kicker);
                cost.Generic += value * perX;
            }

            return cost.ToString();
        }

        for (int i = 0; i < Math.Abs(steps); i++)
        {
            if (steps > 0)
            {
                cost.Add(kicker);
            }
            else
            {
                cost.Take(kicker);
            }
        }

        return cost.ToString();
    }

    /// <summary>
    /// The mana value of the cost but for its X (CR 202.3): what the mana spent on the spell pays before any of it goes to
    /// X. A symbol of two generic mana or a colour counts two; a hybrid or Phyrexian symbol, one.
    /// </summary>
    public static int ManaValueButX(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        Cost cost = Cost.Parse(prompt.Total);
        return cost.ManaValue - (prompt.X * cost.X);
    }

    /// <summary>
    /// The X that <paramref name="mana"/> mana would pay for: what is left of it once the rest of the cost is paid, an X's
    /// worth for each one. Zero when it does not reach past the rest; colours are the payment's to judge.
    /// </summary>
    public static int XPaidBy(PayCostsPromptMessage prompt, int mana)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        int xs = XSymbols(prompt.Total);
        return xs == 0 ? 0 : Clamp(prompt, Math.Max(0, mana - ManaValueButX(prompt)) / xs);
    }

    /// <summary>The symbols of a cost in the server's notation, braces off: "X", "3", "R", "W/U", "2/G", "B/P".</summary>
    private static IEnumerable<string> Symbols(string cost)
    {
        int at = 0;
        while ((at = cost.IndexOf('{', at)) >= 0)
        {
            int end = cost.IndexOf('}', at);
            if (end < 0)
            {
                yield break;
            }

            yield return cost[(at + 1)..end];
            at = end + 1;
        }
    }

    /// <summary>A cost as its X, its generic mana and its other symbols, written back in the order the server writes them.</summary>
    private sealed class Cost
    {
        public int X;
        public int Generic;
        private readonly List<string> _symbols = [];

        public static Cost Parse(string text)
        {
            var cost = new Cost();
            foreach (string symbol in Symbols(text))
            {
                if (symbol == "X")
                {
                    cost.X++;
                }
                else if (int.TryParse(symbol, System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int generic))
                {
                    cost.Generic += generic;
                }
                else
                {
                    cost._symbols.Add(symbol);
                }
            }

            return cost;
        }

        public int ManaValue => Generic + _symbols.Sum(symbol => symbol.StartsWith("2/", StringComparison.Ordinal) ? 2 : 1);

        public void Add(Cost other)
        {
            X += other.X;
            Generic += other.Generic;
            _symbols.AddRange(other._symbols);
        }

        public void Take(Cost other)
        {
            X = Math.Max(0, X - other.X);
            Generic = Math.Max(0, Generic - other.Generic);
            foreach (string symbol in other._symbols)
            {
                _symbols.Remove(symbol);
            }
        }

        /// <summary>
        /// As the server writes a cost: each {X}, the generic mana (or {0} for nothing at all), hybrid symbols, then two-generic
        /// hybrids, Phyrexian symbols, and the coloured and colorless ones in WUBRGC order.
        /// </summary>
        public override string ToString()
        {
            var text = new System.Text.StringBuilder();
            text.Insert(0, "{X}", X);
            if (Generic > 0 || (X == 0 && _symbols.Count == 0))
            {
                text.Append('{').Append(Generic.ToString(System.Globalization.CultureInfo.InvariantCulture)).Append('}');
            }

            foreach (string symbol in _symbols.OrderBy(Rank))
            {
                text.Append('{').Append(symbol).Append('}');
            }

            return text.ToString();
        }

        private static int Rank(string symbol) =>
            symbol.StartsWith("2/", StringComparison.Ordinal) ? 1
            : symbol.EndsWith("/P", StringComparison.Ordinal) ? 2
            : symbol.Contains('/', StringComparison.Ordinal) ? 0
            : 3 + Math.Max(0, "WUBRGC".IndexOf(symbol[0], StringComparison.Ordinal));
    }
}
