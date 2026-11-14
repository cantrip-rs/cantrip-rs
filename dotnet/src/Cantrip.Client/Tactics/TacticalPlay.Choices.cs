using Cantrip.Protocol;

namespace Cantrip.Client.Tactics;

/// <summary>
/// Everything a seat is asked besides what to do with priority and how to fight: what to aim a trigger at, which
/// cards to keep, whether to take a mulligan, whether to do what it "may".
/// </summary>
public sealed partial class TacticalPlay
{
    // --- Aiming a trigger, or a mode -----------------------------------------------------------------------------------

    private AnswerPayload Target(GameClient client, PlayerView view, ChooseTargetPromptMessage prompt)
    {
        if (prompt.Candidates.Count == 0)
        {
            return new TargetsChosenAnswer();
        }

        // "Any number of target creature cards with total mana value X or less" (Fire Lord Sozin's): the most the cards can
        // be worth together within the limit.
        if (prompt.MaximumTotalManaValue is { } most)
        {
            return new TargetsChosenAnswer { Targets = WithinTotal(client, prompt, most) };
        }

        // Spellskite's "change a target ... to this creature" (CR 115.7b): which target the spell or ability lets go of. A seat
        // pays to change a target to its own creature to draw something away from what it would rather keep - itself, then its
        // own permanents - so that is the one let go; otherwise the first.
        if (string.Equals(prompt.Text, CardReading.ChangeTargetLabel, StringComparison.Ordinal))
        {
            TargetMessage spared = prompt.Candidates.FirstOrDefault(c => c.IsPlayer && c.Player == client.Seat)
                ?? prompt.Candidates.FirstOrDefault(c => !c.IsPlayer && client.Object(c.Object)?.Controller == client.Seat)
                ?? prompt.Candidates[0];
            return new TargetsChosenAnswer { Targets = [spared] };
        }

        EffectRead effect = Reading(client, prompt.Text, prompt.Source, prompt.Ability);

        // A bite deals damage equal to the power of the creature chosen for its first slot, to what its second slot chooses:
        // the damage is that creature's power, and so the one dealt it is the one worth killing with that much.
        string plain = CardReading.Plain(prompt.Text);
        bool biter = plain.Contains(CardReading.BiterLabel, StringComparison.Ordinal);
        if (plain.Contains(CardReading.VictimLabel, StringComparison.Ordinal))
        {
            effect = new EffectRead(EffectKind.Damage, _biter is { } dealing && client.Object(dealing) is { } source ? Math.Max(source.Power ?? 0, 0) : 0);
        }

        (List<TargetMessage> targets, _) = Pick(client, view, effect, prompt.Candidates, prompt.Count, prompt.Optional);
        if (biter)
        {
            _biter = targets.Count > 0 && !targets[0].IsPlayer ? targets[0].Object : null;
        }

        // A slot that takes several wants that many (CR 601.2c), or its minimum where it is a printed range ("one or two
        // target creatures"), so it is filled with the best of what is left.
        int fewest = Math.Min(prompt.Minimum > 0 ? prompt.Minimum : prompt.Count, prompt.Candidates.Count);
        if (!prompt.Optional && targets.Count < fewest)
        {
            targets.AddRange(prompt.Candidates.Where(candidate => !targets.Contains(candidate)).Take(fewest - targets.Count));
        }

        return new TargetsChosenAnswer { Targets = targets };
    }

    /// <summary>
    /// The targets of a slot limited to a total mana value (CR 601.2c), at most <see cref="ChooseTargetPromptMessage.Count"/>
    /// of them: those whose mana values fill as much of <paramref name="most"/> as can be filled, the larger cards first among
    /// equals, and every card of mana value 0 besides, which costs nothing of it. A bounded knapsack over the limit, which is
    /// the X just paid and so small. Fire Lord Sozin's puts the cards onto the battlefield under this seat's control, so more
    /// is better; only that slot has a total today.
    /// </summary>
    private static List<TargetMessage> WithinTotal(GameClient client, ChooseTargetPromptMessage prompt, int most)
    {
        int limit = Math.Max(0, Math.Min(most, 1000));
        int Value(int i) => prompt.ManaValues.ElementAtOrDefault(i);
        List<int> positive = [.. Enumerable.Range(0, prompt.Candidates.Count)
            .Where(i => Value(i) > 0)
            .OrderByDescending(Value)
            .ThenByDescending(i => client.Object(prompt.Candidates[i].Object)?.Power ?? 0)];

        // best[t]: the first set found whose values add up to exactly t (0/1 knapsack, the larger cards tried first).
        List<int>?[] best = new List<int>?[limit + 1];
        best[0] = [];
        foreach (int i in positive)
        {
            for (int total = limit; total >= Value(i); total--)
            {
                if (best[total] is null && best[total - Value(i)] is { } before)
                {
                    best[total] = [.. before, i];
                }
            }
        }

        List<int> chosen = [.. Enumerable.Range(0, limit + 1).Reverse().Select(t => best[t]).First(set => set is not null)!,
            .. Enumerable.Range(0, prompt.Candidates.Count).Where(i => Value(i) == 0)];
        return [.. chosen.Take(prompt.Count).Select(i => prompt.Candidates[i])];
    }

    /// <summary>
    /// What is being aimed: the text the prompt came with when it has one, and otherwise the line of the source that
    /// talks about a target.
    /// </summary>
    private static EffectRead Reading(GameClient client, string text, ViewHandle source, ViewHandle ability)
    {
        if (CardReading.Effect(text) is { Kind: not EffectKind.Unknown } stated)
        {
            return stated;
        }

        foreach (ViewHandle handle in new[] { ability, source })
        {
            if (client.Object(handle) is not { } card)
            {
                continue;
            }

            foreach (string line in CardReading.Text(card).Split('\n'))
            {
                if (line.Contains("target", StringComparison.OrdinalIgnoreCase)
                    && CardReading.Effect(line) is { Kind: not EffectKind.Unknown } read)
                {
                    return read;
                }
            }
        }

        // Unread, and so taken to be something done to an opponent — unless the prompt's own words say otherwise.
        return CardReading.Plain(text).Contains("you control", StringComparison.Ordinal) ? new EffectRead(EffectKind.Help) : default;
    }

    /// <summary>
    /// Removes counters from among this seat's permanents as a cost (CR 118.3), one at a time, each the one that costs least to
    /// lose: a -1/-1 or stun counter first, whose loss helps; then a counter of another kind; a +1/+1 counter last, and one whose
    /// loss would leave its creature dead only when nothing else is left.
    /// </summary>
    private AnswerPayload RemoveCounters(GameClient client, RemoveCountersPromptMessage prompt)
    {
        int[] taken = new int[prompt.Pools.Count];
        for (int i = 0; i < prompt.Total; i++)
        {
            int best = -1;
            double cheapest = double.MaxValue;
            for (int p = 0; p < prompt.Pools.Count; p++)
            {
                if (taken[p] >= prompt.Pools[p].Count)
                {
                    continue;
                }

                double cost = CounterLoss(client, prompt, taken, p);
                if (cost < cheapest)
                {
                    cheapest = cost;
                    best = p;
                }
            }

            if (best < 0)
            {
                return prompt.Default();
            }

            taken[best]++;
        }

        return new CountersRemovedAnswer
        {
            Removals = [.. Enumerable.Range(0, prompt.Pools.Count).Where(p => taken[p] > 0).Select(p =>
                new CounterPoolMessage { Object = prompt.Pools[p].Object, Counter = prompt.Pools[p].Counter, Count = taken[p] })],
        };
    }

    /// <summary>What losing one more counter of pool <paramref name="p"/> costs, with those already <paramref name="taken"/>.</summary>
    private double CounterLoss(GameClient client, RemoveCountersPromptMessage prompt, int[] taken, int p)
    {
        CounterPoolMessage pool = prompt.Pools[p];
        if (pool.Counter is "-1/-1" or "stun")
        {
            return -1;
        }

        if (pool.Counter != "+1/+1")
        {
            return 2;
        }

        // A +1/+1 counter is a point of power and toughness; the last ones a creature's toughness stands on, the creature.
        ObjectView? creature = client.Object(pool.Object);
        // Its toughness as the counters already taken leave it: each +1/+1 one less, each -1/-1 one more.
        int change = Enumerable.Range(0, prompt.Pools.Count)
            .Where(q => prompt.Pools[q].Object == pool.Object)
            .Sum(q => prompt.Pools[q].Counter switch { "+1/+1" => -taken[q], "-1/-1" => taken[q], _ => 0 });
        bool dies = creature is { Toughness: { } toughness } && toughness + change - 1 <= creature.DamageMarked;
        return dies ? 10 + _model.Worth(creature!) : 3;
    }

    /// <summary>
    /// Divides damage among targets (CR 601.2d): lethal to each creature worth killing, the ones worth most per point
    /// first, and whatever is left at the opponent, or failing that at the last creature chosen.
    /// </summary>
    private AnswerPayload Divide(GameClient client, PlayerView view, ChooseDividedTargetsPromptMessage prompt)
    {
        if (prompt.Total <= 0 || prompt.Candidates.Count == 0)
        {
            return prompt.Default();
        }

        int most = prompt.MaxTargets > 0 ? prompt.MaxTargets : prompt.Candidates.Count;
        int left = prompt.Total;
        List<TargetShareMessage> shares = [];

        // One target is kept back for the opponent's face only where the face is on offer.
        int rivalSeat = Rival(client, view);
        bool faceOffered = prompt.Candidates.Exists(candidate => candidate.IsPlayer && candidate.Player == rivalSeat);
        int creatures = faceOffered ? most - 1 : most;

        foreach ((TargetMessage target, ObjectView creature, int needed) in prompt.Candidates
            .Where(candidate => !candidate.IsPlayer)
            .Select(candidate => (candidate, client.Object(candidate.Object)))
            .Where(pair => pair.Item2 is { } card && !Mine(client, card) && IsCreature(card))
            .Select(pair => (pair.candidate, pair.Item2!, Math.Max((pair.Item2!.Toughness ?? 0) - pair.Item2.DamageMarked, 1)))
            .OrderByDescending(triple => _model.Worth(triple.Item2) / triple.Item3))
        {
            if (shares.Count >= creatures || needed > left)
            {
                continue;
            }

            shares.Add(new TargetShareMessage { Target = target, Amount = needed });
            left -= needed;
        }

        if (left > 0)
        {
            int rival = Rival(client, view);
            TargetMessage? face = prompt.Candidates.Find(candidate => candidate.IsPlayer && candidate.Player == rival);

            if (face is not null && shares.Count < most)
            {
                shares.Add(new TargetShareMessage { Target = face, Amount = left });
            }
            else if (shares.Count > 0)
            {
                shares[^1].Amount += left;
            }
            else
            {
                (List<TargetMessage> targets, _) = Pick(client, view, new EffectRead(EffectKind.Damage, prompt.Total), prompt.Candidates, 1, optional: false);
                if (targets.Count == 0)
                {
                    return prompt.Default();
                }

                shares.Add(new TargetShareMessage { Target = targets[0], Amount = left });
            }
        }

        return new TargetsDividedAnswer { Shares = shares };
    }

    /// <summary>
    /// The offered creature type that most of this seat's creatures and creature cards in hand have, the earlier offered on
    /// a tie (the first eligible when none has any).
    /// </summary>
    /// <summary>What a miracle option costs: the symbols of its cost, which the server words already reduced ("Miracle {W},
    /// from Aminatou, Veil Piercer"); what is said in brackets after them (X's reduction) is not mana.</summary>
    private static int MiracleCost(string option)
    {
        string text = option.Split(", ")[0];
        int aside = text.IndexOf(" (", StringComparison.Ordinal);
        return CardReading.ManaValue(aside < 0 ? text : text[..aside]);
    }

    /// <summary>
    /// The offered type of mana ("{W}" ... "{C}") whose color the most nonland cards in this seat's hand have, the earlier
    /// offered on a tie; colorless counts as wanted by none, so it is taken only when no offered color is.
    /// </summary>
    private static int ManaTypeWanted(GameClient client, ChooseModesPromptMessage prompt)
    {
        List<ColorView> wanted = [.. client.MyHand().Where(card => !IsLand(card)).SelectMany(card => card.Colors)];
        int best = prompt.Eligible[0];
        int most = -1;
        foreach (int mode in prompt.Eligible)
        {
            ColorView? color = prompt.Modes[mode] switch
            {
                "{W}" => ColorView.White,
                "{U}" => ColorView.Blue,
                "{B}" => ColorView.Black,
                "{R}" => ColorView.Red,
                "{G}" => ColorView.Green,
                _ => null,
            };
            int count = color is { } c ? wanted.Count(w => w == c) : -1;
            if (count > most)
            {
                (best, most) = (mode, count);
            }
        }

        return best;
    }

    /// <summary>The life a "Lose N life" choice costs, or 0 for any other.</summary>
    private static int LifeLost(string mode)
    {
        System.Text.RegularExpressions.Match life = System.Text.RegularExpressions.Regex.Match(mode, @"^Lose (\d+) life$");
        return life.Success ? int.Parse(life.Groups[1].Value, System.Globalization.CultureInfo.InvariantCulture) : 0;
    }

    private static int MostCommonCreatureType(GameClient client, ChooseModesPromptMessage prompt)
    {
        List<ObjectView> mine = [.. Creatures(client, client.Seat), .. client.MyHand().Where(IsCreature)];
        int best = prompt.Eligible[0];
        int most = 0;
        foreach (int mode in prompt.Eligible)
        {
            string type = prompt.Modes[mode];
            int count = mine.Count(card => card.Subtypes.Exists(subtype => string.Equals(subtype, type, StringComparison.OrdinalIgnoreCase)));
            if (count > most)
            {
                (best, most) = (mode, count);
            }
        }

        return best;
    }

    /// <summary>The modes worth most on this board, as many as are asked for.</summary>
    private AnswerPayload Modes(GameClient client, PlayerView view, ChooseModesPromptMessage prompt)
    {
        // Which Phyrexian symbols to pay with life (CR 107.4f, 601.2b).
        if (prompt.AdditionalCost && prompt.Count == 0 && prompt.Modes.Count > 0 && prompt.Modes.TrueForAll(mode => LifeIn(mode) > 0))
        {
            return LifeForPhyrexian(client, view, prompt);
        }

        // An optional additional cost that gives something up for something: collect evidence (CR 701.59a) pays cards of the
        // graveyard for what the spell does "if evidence was collected" (Deadly Cover-Up).
        if (prompt.AdditionalCost && prompt.Count == 1 && prompt.Modes.Count == 2 && prompt.Eligible.Contains(1)
            && prompt.Modes[1].StartsWith("Collect evidence", StringComparison.Ordinal))
        {
            return new ModesChosenAnswer { Modes = [CollectsEvidence(client, view) ? 1 : 0] };
        }

        // The faces a card may be cast as (CR 709.3): each worth what it does or, for a permanent, what it is, the dearer
        // of two equal ones first.
        if (prompt.Eligible.Count > 0 && prompt.Texts.Count == prompt.Modes.Count && prompt.Modes.Count > 1)
        {
            int face = prompt.Eligible
                .OrderByDescending(mode => FaceWorth(client, view, prompt.Modes[mode], prompt.Texts[mode]))
                .ThenByDescending(mode => CardReading.ManaValue(prompt.Modes[mode]))
                .First();
            return new ModesChosenAnswer { Modes = [face] };
        }

        // An additional cost dressed as a choice ("discard a card or pay 3 life"), or a room of a dungeon: the
        // prompt's own order is as good as this seat can do.
        // "You may behold a Dragon" (CR 701.4a): offered only when there is one to choose or reveal, and it costs nothing
        // but showing a card, so it is taken for what it adds. "You may blight 1" stays declined: it shrinks a creature of
        // its own for a bonus, which this seat does not weigh yet (a known limitation: Requiting Hex's and Pyrrhic Strike's optional blight is never paid).
        if (prompt.AdditionalCost && prompt.CostKind == "Behold" && prompt.Count == 1 && prompt.Modes.Count == 2 && prompt.Eligible.Contains(1))
        {
            return new ModesChosenAnswer { Modes = [1] };
        }

        // "You may sacrifice an artifact or discard a card. If you do, draw a card" (Chandra, Spark Hunter's +2): never worth an
        // artifact, and a card of the hand only when the hand is full enough that the worst of it is a spare; otherwise Neither.
        if (!prompt.AdditionalCost && prompt.Heading.Length == 0 && prompt.Count == 1
            && prompt.Modes.FindIndex(mode => mode == "Neither") is int neither and >= 0
            && prompt.Modes.Exists(mode => mode == "Sacrifice an artifact" || mode == "Discard a card"))
        {
            int discard = prompt.Modes.FindIndex(mode => mode == "Discard a card");
            bool spare = discard >= 0 && prompt.Eligible.Contains(discard) && client.MyHand().Count >= 5;
            return new ModesChosenAnswer { Modes = [spare ? discard : neither] };
        }

        // "Each opponent loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card"
        // (Rottenmouth Viper), asked of this seat once for each time: the life while losing it costs less than a card does
        // (never the last of it), otherwise a card from the hand, otherwise the least of its permanents (which the choice of
        // permanent after it picks).
        if (!prompt.AdditionalCost && prompt.Heading.Length == 0 && prompt.Count == 1
            && prompt.Modes.FindIndex(mode => LifeLost(mode) > 0) is int lose and >= 0
            && prompt.Modes.Exists(mode => mode.StartsWith("Sacrifice a", StringComparison.Ordinal) || mode == "Discard a card"))
        {
            int mine = LifeOf(view, client.Seat);
            int loss = LifeLost(prompt.Modes[lose]);
            double hurts = _model.Life(mine) - _model.Life(mine - loss);
            int discard = prompt.Modes.FindIndex(mode => mode == "Discard a card");
            int sacrifice = prompt.Modes.FindIndex(mode => mode.StartsWith("Sacrifice a", StringComparison.Ordinal));
            int chosen = mine - loss > 0 && hurts < _model.Card ? lose
                : discard >= 0 && prompt.Eligible.Contains(discard) ? discard
                : sacrifice >= 0 && prompt.Eligible.Contains(sacrifice) ? sacrifice
                : lose;
            return new ModesChosenAnswer { Modes = [chosen] };
        }

        // Discarding one card instead of two (Steamcore Scholar): the cheaper discard, whenever it is offered.
        if (!prompt.AdditionalCost && prompt.Count == 1 && prompt.Modes.Count == 2 && prompt.Eligible.Contains(0)
            && prompt.Modes[0].StartsWith("Discard an instant or sorcery", StringComparison.Ordinal))
        {
            return new ModesChosenAnswer { Modes = [0] };
        }

        // A text change's words (CR 612): which kind of word, which word to replace and which replaces it.
        if (TextChangeKind(client, prompt) is int kind)
        {
            return new ModesChosenAnswer { Modes = [kind] };
        }

        if (TextChangeSubtype(client, view, prompt) is int word)
        {
            return new ModesChosenAnswer { Modes = [word] };
        }

        // "As this enters, choose a creature type" (Roaming Throne, Chronicle of Victory, Door of Destinies): every such card
        // rewards the type this seat has most of, so the type most common among its creatures on the battlefield and the
        // creature cards in its hand; with none of any offered type, the prompt's own first.
        if (prompt.Heading == "Choose a creature type" && prompt.Eligible.Count > 0)
        {
            return new ModesChosenAnswer { Modes = [MostCommonCreatureType(client, prompt)] };
        }

        // Which miracle ability to reveal a card with (CR 702.94a): each is the same cast at a different cost, so the cheapest,
        // and never one that can't be paid. The cost shown is already reduced; an X in it is left as the bot would announce it.
        if (prompt.Heading == "Choose which miracle to reveal it with" && prompt.Eligible.Count > 0)
        {
            int cheapest = prompt.Eligible
                .OrderBy(mode => prompt.Modes[mode].Contains("unpayable", StringComparison.Ordinal))
                .ThenBy(mode => MiracleCost(prompt.Modes[mode]))
                .First();
            return new ModesChosenAnswer { Modes = [cheapest] };
        }

        // "One mana of any type that land produced" (Mana Flare, Lavaleaper): the type its spells in hand ask for most, as
        // mana of a color chosen is; colorless only when no color offered is asked for.
        if (prompt.Heading == "Choose which type of mana to add" && prompt.Eligible.Count > 0)
        {
            return new ModesChosenAnswer { Modes = [ManaTypeWanted(client, prompt)] };
        }

        if (prompt.Eligible.Count == 0 || prompt.AdditionalCost || prompt.Heading.Length > 0)
        {
            return prompt.Default();
        }

        // The modes it could pay for before the ones it could not (spree), and within each the one worth most: a mode
        // chosen and then given up on while paying is a spell that is never cast.
        List<(int Mode, double Worth)> worths = [.. prompt.Eligible
            .Select(mode => (Mode: mode, Worth: mode < prompt.Modes.Count ? Mode(client, view, prompt.Modes[mode]) : 0))
            .OrderBy(pair => prompt.Affordable.Count > 0 && !prompt.Affordable.Contains(pair.Mode))
            .ThenByDescending(pair => pair.Worth)];
        List<int> ranked = [.. worths.Select(pair => pair.Mode)];

        if (prompt.MayRepeat)
        {
            return new ModesChosenAnswer { Modes = [.. Enumerable.Repeat(ranked[0], prompt.Count)] };
        }

        int take = Math.Max(prompt.Count, 1);

        // "Choose one or both" with nothing to pay for the second: every further mode that is worth doing is free.
        if (!prompt.ExtraModesCost && prompt.Most > take)
        {
            take += worths.Skip(take).Take(prompt.Most - take).Count(pair => pair.Worth > 0);
        }

        return new ModesChosenAnswer { Modes = [.. ranked.Take(take)] };
    }

    /// <summary>
    /// What one face of a card is worth cast: an effect for what it does to the best target there is, and anything else (a
    /// creature, an artifact) for what a card of that cost is worth on the table.
    /// </summary>
    private double FaceWorth(GameClient client, PlayerView view, string name, string text)
    {
        if (CardReading.Effect(text).Kind != EffectKind.Unknown)
        {
            return Mode(client, view, text);
        }

        return 0.3 + _model.Card + (0.1 * Math.Min(CardReading.ManaValue(name), 4));
    }

    /// <summary>
    /// Whether to pay collect evidence for its bonus: only with something in an opponent's graveyard for the bonus to take
    /// (it starts there), and with no card of this seat's graveyard it would rather keep - a creature to bring back, or
    /// anything with flashback, escape or the like - for the cards that pay for it are the best of them.
    /// </summary>
    private static bool CollectsEvidence(GameClient client, PlayerView view)
    {
        if (!view.Objects.Exists(card => card.Zone == ZoneKindView.Graveyard && card.ZoneOwner != client.Seat))
        {
            return false;
        }

        return !view.Objects.Exists(card => card.Zone == ZoneKindView.Graveyard && card.ZoneOwner == client.Seat
            && (IsCreature(card) || CardReading.Text(card).Contains("from your graveyard", StringComparison.OrdinalIgnoreCase)
                || CardReading.Text(card).Contains("flashback", StringComparison.OrdinalIgnoreCase)
                || CardReading.Text(card).Contains("escape", StringComparison.OrdinalIgnoreCase)
                || CardReading.Text(card).Contains("unearth", StringComparison.OrdinalIgnoreCase)));
    }

    // --- Cards ---------------------------------------------------------------------------------------------------------

    /// <summary>
    /// How much this seat wants a card it could have: a land while it is short of them, a spell it could soon cast,
    /// and little for either past that.
    /// </summary>
    private double Wanted(GameClient client, ObjectView card, int landsHeld)
    {
        int lands = Lands(client, client.Seat);

        if (IsLand(card))
        {
            return lands + landsHeld < 4 ? 1.0 : lands + landsHeld < 6 ? 0.3 : 0.05;
        }

        double worth = IsPermanentCard(card) ? _model.WorthOnceCast(card) : _model.Card + (0.1 * Math.Min(card.ManaValue, 4));
        return 0.3 + worth - (0.15 * Math.Max(card.ManaValue - lands - landsHeld - 1, 0));
    }

    /// <summary>
    /// The two cards Ill-Timed Explosion has this seat discard: the card it wants least, and, with it, the other card of most
    /// mana value, since the damage is the greatest mana value of the two. Empty with fewer than two to choose from.
    /// </summary>
    private List<ObjectView> SweepDiscards(GameClient client, IEnumerable<ViewHandle> eligible)
    {
        List<ViewHandle> ranked = Ranked(client, eligible);
        if (ranked.Count < 2 || client.Object(ranked[^1]) is not { } worst)
        {
            return [];
        }

        ObjectView? highest = ranked.Take(ranked.Count - 1)
            .Select(client.Object)
            .OfType<ObjectView>()
            .OrderByDescending(card => card.ManaValue)
            .FirstOrDefault();

        return highest is null ? [] : [highest, worst];
    }

    /// <summary>
    /// Whether Ill-Timed Explosion's damage, the greatest mana value among the two cards <see cref="SweepDiscards"/> would
    /// have this seat discard, kills more creatures across the table than this seat's own (toughness less damage marked, at
    /// most that much).
    /// </summary>
    private bool SweepPays(GameClient client, PlayerView view)
    {
        List<ObjectView> discards = SweepDiscards(client, client.MyHand().Select(card => card.Handle));
        if (discards.Count < 2)
        {
            return false;
        }

        int damage = discards.Max(card => card.ManaValue);
        bool Dies(ObjectView creature) => (creature.Toughness ?? 0) - creature.DamageMarked <= damage;

        return view.Players.Where(player => player.Seat != client.Seat).Sum(player => Creatures(client, player.Seat).Count(Dies))
            > Creatures(client, client.Seat).Count(Dies);
    }

    /// <summary>These cards from the one this seat wants most to the one it wants least.</summary>
    private List<ViewHandle> Ranked(GameClient client, IEnumerable<ViewHandle> cards)
    {
        List<ObjectView> known = [.. cards.Select(client.Object).OfType<ObjectView>()];
        List<ViewHandle> ranked = [];
        int landsHeld = 0;

        // One at a time, because a second land is wanted less than the first.
        while (known.Count > 0)
        {
            ObjectView next = known.MaxBy(card => Wanted(client, card, landsHeld))!;
            known.Remove(next);
            ranked.Add(next.Handle);
            landsHeld += IsLand(next) ? 1 : 0;
        }

        return ranked;
    }

    /// <summary>The cards among these this seat can best do without.</summary>
    private List<ViewHandle> Least(GameClient client, PlayerView view, IReadOnlyList<ViewHandle> cards, int count)
    {
        _ = view;
        List<ViewHandle> ranked = Ranked(client, cards);
        ranked.Reverse();

        // Anything it cannot see is as good as anything else it cannot see.
        ranked.AddRange(cards.Where(card => !ranked.Contains(card)));
        return [.. ranked.Take(Math.Clamp(count, 0, cards.Count))];
    }

    /// <summary>
    /// The creature harmonize taps (CR 702.180a), up to one: none while the mana at hand pays the printed harmonize cost, since the
    /// creature is then worth more untapped (to attack, to block). Short of the mana, a creature whose power (the prompt's, which
    /// is what comes off the cost) leaves the rest payable once its own mana is no longer there (CR 702.180b: tapped for the
    /// cost, it makes none), the one worth least; else the one that comes nearest. Cost increases (Thalia) are not in the
    /// prompt, so they are not counted: the seat then answers as if there were none, and pays what the game asks.
    /// </summary>
    private AnswerPayload TapToReduce(GameClient client, ChooseCardsPromptMessage prompt)
    {
        string text = client.Object(prompt.Source) is { } spell ? CardReading.Text(spell) : string.Empty;
        System.Text.RegularExpressions.Match harmonize = System.Text.RegularExpressions.Regex.Match(text, @"Harmonize ((\{[^}]+\})+)");
        int cost = harmonize.Success ? CardReading.ManaValue(harmonize.Groups[1].Value) : 0;
        int mana = Mana(client);

        if (cost <= mana)
        {
            return new CardsChosenAnswer { Cards = [] };
        }

        List<(ViewHandle Card, int Margin)> options = [.. prompt.Cards.Select((card, index) =>
        {
            ObjectView? creature = client.Object(card);
            int own = creature is { ProducesMana: true } && !creature.SummoningSick ? 1 : 0;
            return (card, mana - own - (cost - (index < prompt.Powers.Count ? prompt.Powers[index] : 0)));
        })];
        List<(ViewHandle Card, int Margin)> enough = [.. options.Where(option => option.Margin >= 0)];
        ViewHandle chosen = enough.Count > 0
            ? enough.MinBy(option => client.Object(option.Card) is { } card ? Worth(client, card) : 0).Card
            : options.MaxBy(option => option.Margin).Card;

        return new CardsChosenAnswer { Cards = [chosen] };
    }

    /// <summary>
    /// The cards an effect exiles to become plotted (CR 702.170d: cast as a sorcery on a later turn without paying its mana
    /// cost): the ones whose mana cost is most worth not paying - the highest mana value first, and among equals one it could
    /// not pay for next turn anyway, then a card that loses nothing by waiting for sorcery timing (not an instant or a card with
    /// flash), then the one it wants most. A land is never plotted (it is played, not cast, so it could never leave exile), nor,
    /// when the choice may be declined, a card of mana value 0, which saves nothing and only waits a turn. A choice that must
    /// be made takes the best of the rest.
    /// </summary>
    private List<ViewHandle> ToPlot(GameClient client, ChooseCardsPromptMessage prompt)
    {
        int nextTurn = Mana(client) + (client.MyHand().Any(IsLand) ? 1 : 0);
        List<ObjectView> known = [.. prompt.Verdict.Addable.Select(client.Object).OfType<ObjectView>()];
        List<ObjectView> ranked = [.. known
            .OrderBy(card => IsLand(card) ? 1 : 0)
            .ThenByDescending(card => card.ManaValue)
            .ThenByDescending(card => card.ManaValue > nextTurn ? 1 : 0)
            .ThenBy(card => card.CardTypes.Contains(CardTypeView.Instant)
                || (CardReading.KeywordsOf(card) & Keywords.Flash) != 0 ? 1 : 0)
            .ThenByDescending(card => Worth(client, card))];
        int most = Math.Min(prompt.Count, prompt.Verdict.Addable.Count);
        List<ViewHandle> chosen = [.. ranked.Where(card => !IsLand(card) && card.ManaValue > 0).Take(most).Select(card => card.Handle)];

        if (!prompt.UpTo || chosen.Count < prompt.Minimum)
        {
            int need = prompt.UpTo ? prompt.Minimum : most;
            chosen.AddRange(ranked.Select(card => card.Handle).Concat(prompt.Verdict.Addable)
                .Distinct().Where(card => !chosen.Contains(card)).Take(need - chosen.Count));
        }

        if (Narrating && chosen.Count > 0)
        {
            Note($"plots {string.Join(", ", chosen.Select(card => client.Object(card)?.Name ?? "a card"))} to cast for free later");
        }

        return chosen;
    }

    /// <summary>
    /// The eligible cards worth most together whose mana values add up to <paramref name="budget"/> or less and that are no
    /// more than the prompt's count: each worth what it would be once in play. Every set is weighed when there are few
    /// enough cards (a look at the top eight is 256); beyond that, the most valuable that still fit, one at a time. A card
    /// worth nothing is left out.
    /// </summary>
    private List<ViewHandle> WorthMostWithin(GameClient client, ChooseCardsPromptMessage prompt, int budget)
    {
        List<ViewHandle> eligible = prompt.Verdict.Addable;
        int n = eligible.Count;
        double[] worth = [.. eligible.Select(card => client.Object(card) is { } known ? Math.Max(0, _model.WorthOnceCast(known)) : 0)];
        int[] value = [.. eligible.Select(card => prompt.Cards.IndexOf(card) is >= 0 and var at && at < prompt.ManaValues.Count ? prompt.ManaValues[at] : 0)];
        int most = Math.Min(prompt.Count, n);

        if (n > 16)
        {
            List<ViewHandle> taken = [];
            int left = budget;
            foreach (int index in Enumerable.Range(0, n).OrderByDescending(index => worth[index]))
            {
                if (taken.Count < most && worth[index] > 0 && value[index] <= left)
                {
                    taken.Add(eligible[index]);
                    left -= value[index];
                }
            }

            return taken;
        }

        int best = 0;
        double bestWorth = 0;
        for (int set = 1; set < 1 << n; set++)
        {
            int count = 0, total = 0;
            double sum = 0;
            for (int index = 0; index < n; index++)
            {
                if ((set & 1 << index) != 0)
                {
                    count++;
                    total += value[index];
                    sum += worth[index];
                }
            }

            if (count <= most && total <= budget && sum > bestWorth)
            {
                best = set;
                bestWorth = sum;
            }
        }

        return [.. Enumerable.Range(0, n).Where(index => (best & 1 << index) != 0).Select(index => eligible[index])];
    }

    private AnswerPayload Cards(GameClient client, PlayerView view, ChooseCardsPromptMessage prompt)
    {
        // A choice that has to add up to a power (saddle, crew): the creatures that reach it and give up least, where
        // the prompt's own answer is every creature there is.
        if (prompt.MinimumPower > 0 && Policy("crew") > 0
            && Crewing(client, prompt.Cards, prompt.Powers, prompt.MinimumPower) is { Count: > 0 } crew)
        {
            return new CardsChosenAnswer { Cards = crew };
        }

        // Collect evidence (CR 701.59a): the most valuable cards first, so the fewest cards leave the graveyard.
        if (prompt.MinimumManaValue > 0)
        {
            List<ViewHandle> taken = [];
            int total = 0;

            foreach (int index in Enumerable.Range(0, prompt.Cards.Count)
                .OrderByDescending(index => index < prompt.ManaValues.Count ? prompt.ManaValues[index] : 0))
            {
                if (total >= prompt.MinimumManaValue)
                {
                    break;
                }

                taken.Add(prompt.Cards[index]);
                total += index < prompt.ManaValues.Count ? prompt.ManaValues[index] : 0;
            }

            return total >= prompt.MinimumManaValue ? new CardsChosenAnswer { Cards = taken } : prompt.Default();
        }

        // "Up to two creature cards with total mana value 6 or less" (CR 202.3): cards taken, onto the battlefield or into a hand,
        // so the set worth most together among those that fit the limit and the count - never a set over it, which the engine
        // refuses. A big creature alone can beat two small ones, so the sets are weighed whole rather than best-first.
        if (prompt.MaximumTotalManaValue is { } budget)
        {
            return new CardsChosenAnswer { Cards = WorthMostWithin(client, prompt, budget) };
        }

        // Otherwise that, an order or a vote: the prompt's own answer.
        if (prompt.MinimumPower > 0 || prompt.Verdict.Addable.Count == 0
            || prompt.Purpose is CardChoicePurposeView.OrderSimultaneous or CardChoicePurposeView.Vote or CardChoicePurposeView.Splice)
        {
            return prompt.Default();
        }

        // Harmonize (CR 702.180a): the creature tapped to take its power off the generic mana of the cost. A tapped creature can
        // neither attack nor block until it untaps, so none is tapped unless the mana for the cost is not there without it.
        if (prompt.Purpose == CardChoicePurposeView.TapToReduce)
        {
            return TapToReduce(client, prompt);
        }

        // A prevention shield too small for all the damage two or more sources would deal at once (CR 615.7): the damage that
        // would hurt most is prevented first - deathtouch's or lifelink's, when the shield has enough
        // left to prevent all of it (any of it dealt is the whole harm), then the most damage.
        if (prompt.Purpose == CardChoicePurposeView.PreventDamageFrom)
        {
            ViewHandle first = prompt.Verdict.Addable
                .OrderByDescending(card => client.Object(card) is { } source && (source.Power ?? 0) <= prompt.PreventLeft
                    && (CardReading.KeywordsOf(source) & (Keywords.Deathtouch | Keywords.Lifelink)) != 0 ? 1 : 0)
                .ThenByDescending(card => client.Object(card)?.Power ?? 0)
                .First();
            return new CardsChosenAnswer { Cards = [first] };
        }

        // Banding (CR 702.22c, 702.22k): the band's controller divides the damage of whatever blocks it among the band, so the
        // creature with banding can take a blocker's damage for an attacker worth more. The one attacker without banding worth
        // most, and worth more than the creature with banding, joins it; the other creatures with banding stay apart, each
        // blocked on its own. With none worth protecting, no band: a band is blocked as one, so it only loses evasion.
        if (prompt.Purpose == CardChoicePurposeView.Band)
        {
            double own = client.Object(prompt.Source) is { } leader ? _model.Worth(leader) : 0;
            ObjectView? best = prompt.WithoutBanding
                .Select(client.Object)
                .OfType<ObjectView>()
                .OrderByDescending(creature => _model.Worth(creature))
                .FirstOrDefault();
            if (best is not null && _model.Worth(best) > own)
            {
                if (Narrating)
                {
                    Note($"{best.Name} bands with {client.Object(prompt.Source)?.Name}, which can take its blockers' damage");
                }

                return new CardsChosenAnswer { Cards = [best.Handle] };
            }

            return new CardsChosenAnswer();
        }

        // A card exiled to become plotted (CR 702.170d) is gained, not given up: cast later for free.
        if (prompt.Purpose == CardChoicePurposeView.Plot)
        {
            return new CardsChosenAnswer { Cards = ToPlot(client, prompt) };
        }

        int most = Math.Min(prompt.Count, prompt.Verdict.Addable.Count);

        // Station's creature (CR 702.184a): its power is the counters the permanent gets, so the one Station chose, or else
        // the one of most power - not the one worth least, as another cost that taps would have it.
        if (prompt.Purpose == CardChoicePurposeView.TapToPay && client.Object(prompt.Source) is { } stationed
            && CardReading.LastStationThreshold(CardReading.Text(stationed)) > 0)
        {
            ViewHandle chosen = prompt.Verdict.Addable.Contains(_stationWith) ? _stationWith
                : prompt.Verdict.Addable.OrderByDescending(card => client.Object(card)?.Power ?? 0).First();
            _stationWith = ViewHandle.None;
            return new CardsChosenAnswer { Cards = [chosen] };
        }

        // Ill-Timed Explosion: the two cards that make the damage biggest for the least loss (see SweepDiscards).
        if (prompt.Purpose == CardChoicePurposeView.Discard && !prompt.UpTo && most == 2 && client.Object(prompt.Source)?.Name == "Ill-Timed Explosion")
        {
            return new CardsChosenAnswer { Cards = [.. SweepDiscards(client, prompt.Verdict.Addable).Select(card => card.Handle)] };
        }

        // What to behold (CR 701.4a): a permanent on the battlefield before a card from the hand, since revealing a card tells the
        // opponent what is in it and choosing a permanent tells them nothing.
        if (prompt.Purpose == CardChoicePurposeView.Behold)
        {
            List<ViewHandle> beheld = [.. prompt.Verdict.Addable.OrderBy(card => client.Object(card)?.Zone == ZoneKindView.Battlefield ? 0 : 1)];
            return new CardsChosenAnswer { Cards = [.. beheld.Take(most)] };
        }

        // Craft's materials (CR 702.167a) are exiled for good: the fewest the ability takes (its minimum for "one or more"),
        // cards from the graveyard before permanents still at work, and among each the ones worth least.
        if (prompt.Purpose == CardChoicePurposeView.Craft)
        {
            int need = prompt.UpTo ? Math.Max(prompt.Minimum, 1) : prompt.Count;
            List<ViewHandle> materials = [.. prompt.Verdict.Addable
                .OrderBy(card => client.Object(card)?.Zone == ZoneKindView.Battlefield ? 1 : 0)
                .ThenBy(card => client.Object(card) is { } known ? Worth(client, known) : 0)];
            return new CardsChosenAnswer { Cards = [.. materials.Take(need)] };
        }

        // A sacrifice whose power is what the spell deals (Fling, Kazuul's Fury: "the sacrificed creature's power"): the strongest
        // creature it may give up, the one worth least among equals - a 0-power one would make the spell do nothing.
        if (prompt.Purpose == CardChoicePurposeView.Sacrifice && prompt.Count == 1
            && client.Object(prompt.Source) is { } flung
            && CardReading.Plain(CardReading.Text(flung)).Contains("sacrificed creature's power", StringComparison.Ordinal))
        {
            List<ViewHandle> strongest = [.. prompt.Verdict.Addable
                .OrderByDescending(card => client.Object(card)?.Power ?? 0)
                .ThenBy(card => client.Object(card) is { } known ? Worth(client, known) : 0)];
            return new CardsChosenAnswer { Cards = [.. strongest.Take(1)] };
        }

        // Giving something up, or taking something: which end of the ranking is wanted. A card revealed to be kept or put into play
        // (Break Out) is taken, so the ordinary best-first ranking stands.
        bool losing = prompt.Purpose is CardChoicePurposeView.Discard or CardChoicePurposeView.Sacrifice or CardChoicePurposeView.Exile
            or CardChoicePurposeView.PutOnBottom or CardChoicePurposeView.PutIntoGraveyard or CardChoicePurposeView.PutOnTop
            // Sylvan Library's two cards are paid for or put back, so the ones worth least are chosen and the best kept free.
            or CardChoicePurposeView.PayLifeOrPutOnTop
            or CardChoicePurposeView.ExileToPay or CardChoicePurposeView.TapToPay or CardChoicePurposeView.Tap or CardChoicePurposeView.Destroy
            // Blight (CR 701.68a) shrinks its own creature: the one worth least takes the counters. Behold loses nothing, so the
            // ordinary best-first ranking stands (every candidate already is the quality the cost names).
            or CardChoicePurposeView.Blight
            // Counters put on its own permanent (empower Jace, amass) help it, so the best one takes them, as the ordinary ranking
            // does; harmful ones (-1/-1, stun) go on the one worth least.
            || prompt.Purpose == CardChoicePurposeView.PutCounters && prompt.Counter is "-1/-1" or "stun";

        List<ObjectView> known = [.. prompt.Verdict.Addable.Select(client.Object).OfType<ObjectView>()];
        bool theirs = known.Count > 0 && known.TrueForAll(card => card.Owner != client.Seat && card.Controller != client.Seat);
        bool board = known.Count > 0 && known.TrueForAll(card => card.Zone == ZoneKindView.Battlefield);

        // Returning its own permanent to hand is something given up — a bounce land's cost — where returning a card
        // from a graveyard is something gained.
        losing |= prompt.Purpose == CardChoicePurposeView.ReturnToHand && board && !theirs;

        // Onto the battlefield for nothing (Sneak Attack, Exhume): the one worth most there, whatever it would cost.
        // From the hand, by the reckoning the cheat was used for: Flash's has to be one it can pay for, and none of
        // them one it could as well have cast. Lands are not chosen this way (Uro's land wants a colour).
        bool free = Cheating && prompt.Purpose == CardChoicePurposeView.PutOntoBattlefield && !board && known.Count > 0
            && known.TrueForAll(card => IsPermanentCard(card) && !IsLand(card));

        if (free && client.Object(prompt.Source) is { } cheat && FromHandText(CardReading.Plain(CardReading.Text(cheat)))
            && known.TrueForAll(card => card.Zone == ZoneKindView.Hand))
        {
            List<ViewHandle> worth = [.. Cheated(client, view, CardReading.Text(cheat), ViewHandle.None, Mana(client))
                .Where(one => prompt.Verdict.Addable.Contains(one.Creature.Handle))
                .OrderByDescending(one => one.Net)
                .Select(one => one.Creature.Handle)];

            if (worth.Count > 0 || prompt.UpTo)
            {
                return new CardsChosenAnswer { Cards = [.. worth.Take(Math.Min(prompt.Count, prompt.Verdict.Addable.Count))] };
            }
        }

        List<ViewHandle> ranked = board
            ? [.. known.OrderByDescending(card => Worth(client, card)).Select(card => card.Handle)]
            : free ? [.. known.OrderByDescending(_model.WorthOnceCast).Select(card => card.Handle)]
            : Ranked(client, prompt.Verdict.Addable);

        ranked.AddRange(prompt.Verdict.Addable.Where(card => !ranked.Contains(card)));

        // The worst of its own when it is losing them, the best of an opponent's when they are: Duress takes the
        // card worth most, an edict gives up the creature worth least.
        if (losing != theirs && !free)
        {
            ranked.Reverse();
        }

        int take = most;
        if (prompt.UpTo)
        {
            if (losing && !theirs)
            {
                // "You may discard a card": only for something, and only a card it can spare — a land it has no use for.
                bool pays = client.Object(prompt.Source) is { } source
                    && CardReading.Plain(CardReading.Text(source)) is var text
                    && (text.Contains("when you do", StringComparison.Ordinal) || text.Contains("if you do", StringComparison.Ordinal));
                bool spare = client.Object(ranked[0]) is { } worst && IsLand(worst) && Lands(client, client.Seat) >= 4;
                take = pays && spare && prompt.Purpose == CardChoicePurposeView.Discard ? 1 : 0;

                // Chrome Mox was cast to be imprinted: the card wanted least that has a colour for it to make. (The
                // prompt's Source is the trigger, whose text is the imprint line. Mox Diamond's discard is not "up to":
                // the ordinary path above discards the land wanted least.)
                if (Policy("imprint") > 0 && Policy("rocks") > 0 && client.Object(prompt.Source) is { } mox
                    && CardReading.Plain(CardReading.Text(mox)).Contains("exile a nonartifact, nonland card from your hand", StringComparison.Ordinal))
                {
                    ranked = [.. ranked.Where(card => client.Object(card) is { Colors.Count: > 0 }), .. ranked.Where(card => client.Object(card) is not { Colors.Count: > 0 })];
                    take = Math.Min(1, ranked.Count);
                }
            }
        }

        // "You may cast it": a card chosen once this step and offered again was not cast - its cost could not be paid,
        // its mode had no target - and choosing it again only asks the same question again.
        if (prompt.Purpose is CardChoicePurposeView.Cast or CardChoicePurposeView.CastFree or CardChoicePurposeView.CastCopyFree or CardChoicePurposeView.CastCopyFreeWithin or CardChoicePurposeView.Play)
        {
            ranked.RemoveAll(card => _offered.Contains(card.Value));
            take = Math.Min(take, ranked.Count);

            if (take == 0 && !prompt.UpTo)
            {
                return prompt.Default();
            }

            foreach (ViewHandle chosen in ranked.Take(take))
            {
                _offered.Add(chosen.Value);
            }
        }

        _ = view;
        return new CardsChosenAnswer { Cards = [.. ranked.Take(take)] };
    }

    /// <summary>
    /// From a library: a land that makes a color this seat is short of when it is lands on offer, and otherwise the
    /// card it wants most.
    /// </summary>
    private AnswerPayload Search(GameClient client, PlayerView view, SearchLibraryPromptMessage prompt)
    {
        _ = view;
        List<ObjectView> eligible = [.. prompt.Verdict.Addable.Select(client.Object).OfType<ObjectView>()];
        int most = Math.Min(prompt.Count, prompt.Verdict.Addable.Count);

        if (eligible.Count == 0 || most <= 0)
        {
            return new CardsChosenAnswer { Cards = [.. prompt.Verdict.Addable.Take(most)] };
        }

        if (eligible.TrueForAll(IsLand))
        {
            HashSet<ColorView> have = [.. client.BattlefieldOf(client.Seat).Where(IsLand).SelectMany(Makes)];
            HashSet<ColorView> want = [.. client.MyHand().Where(card => !IsLand(card)).SelectMany(card => card.Colors)];

            return new CardsChosenAnswer
            {
                Cards = [.. eligible
                    .OrderByDescending(land => Makes(land).Count(color => want.Contains(color) && !have.Contains(color)))
                    .ThenByDescending(land => Makes(land).Count)
                    .ThenBy(land => CardReading.Plain(CardReading.Text(land)).Contains("enters tapped", StringComparison.Ordinal))
                    .Take(most)
                    .Select(land => land.Handle)],
            };
        }

        // Onto the battlefield, nothing is paid for it: the permanent worth most there, whatever it would cost to cast
        // (Tinker's Colossus, Natural Order's biggest creature), not the one this seat could soonest afford.
        if (prompt.Destination == SearchDestinationView.Battlefield && Policy("fetch") > 0 && eligible.Exists(card => IsPermanentCard(card) && !IsLand(card)))
        {
            return new CardsChosenAnswer
            {
                Cards = [.. eligible.Where(card => IsPermanentCard(card) && !IsLand(card))
                    .OrderByDescending(_model.WorthOnceCast).Take(most).Select(card => card.Handle)],
            };
        }

        // Into the graveyard is where a card goes to be brought back: the biggest creature there is.
        if (prompt.Destination == SearchDestinationView.Graveyard && eligible.Exists(IsCreature))
        {
            return new CardsChosenAnswer
            {
                Cards = [.. eligible.Where(IsCreature).OrderByDescending(_model.WorthOnceCast).Take(most).Select(card => card.Handle)],
            };
        }

        return new CardsChosenAnswer { Cards = [.. Ranked(client, prompt.Verdict.Addable).Take(most)] };
    }

    /// <summary>
    /// A scry or a surveil: a card it wants stays on top, one it does not goes wherever else the prompt offers.
    /// </summary>
    private AnswerPayload Distribute(GameClient client, PlayerView view, DistributeCardsPromptMessage prompt)
    {
        _ = view;
        int keep = prompt.Destinations.FindIndex(destination => destination.Kind is CardDestinationKindView.LibraryTop or CardDestinationKindView.Hand);
        int away = prompt.Destinations.FindLastIndex(destination =>
            destination.Kind is CardDestinationKindView.LibraryBottom or CardDestinationKindView.Graveyard);

        if (keep < 0 || away < 0 || keep == away)
        {
            return AutoPlay.Distribute(client, prompt);
        }

        int held = client.MyHand().Count(IsLand);
        var answer = new CardsDistributedAnswer { Placements = [.. prompt.Destinations.Select(_ => new List<ViewHandle>())] };

        foreach (ViewHandle handle in prompt.Cards)
        {
            bool wanted = client.Object(handle) is not { } card || Wanted(client, card, held) >= 0.3;
            answer.Placements[wanted ? keep : away].Add(handle);
        }

        return prompt.Problem(answer) is null ? answer : AutoPlay.Distribute(client, prompt);
    }

    /// <summary>
    /// Keeps a hand it can play: two to five lands of seven, and anything at all once it is down to five cards,
    /// where another mulligan costs more than a clumsy hand does.
    /// </summary>
    private static AnswerPayload Mulligan(GameClient client, MulliganPromptMessage prompt)
    {
        IReadOnlyList<ObjectView> hand = client.MyHand();
        int lands = hand.Count(IsLand);

        bool keep = prompt.Keeps <= 5 || hand.Count == 0
            || (prompt.Keeps >= 7 ? lands is >= 2 and <= 5 : lands is >= 2 and <= 4);

        return new MayChosenAnswer { Accept = !keep };
    }

    // --- Yes or no, who, and what color --------------------------------------------------------------------------------

    /// <summary>What the Food this seat controls that is worth least is worth, or nothing when it controls no Food.</summary>
    private double? CheapestFood(GameClient client)
    {
        List<double> foods = [.. client.BattlefieldOf(client.Seat).Where(card => card.Subtypes.Contains("Food")).Select(card => Worth(client, card))];
        return foods.Count == 0 ? null : foods.Min();
    }

    /// <summary>What the opponents' permanents a spell or ability on the stack targets are worth to them, together.</summary>
    private double AimedAt(GameClient client, ViewHandle spell) =>
        client.Object(spell)?.Targets
            .Select(target => client.Object(target.Object))
            .OfType<ObjectView>()
            .Where(permanent => permanent.Zone == ZoneKindView.Battlefield && permanent.Controller != client.Seat)
            .Sum(permanent => Worth(client, permanent)) ?? 0;

    /// <summary>
    /// Whether the next lore counter the Saga gets would be its last chapter's (CR 714.2b, 714.4): its lore counters are one short of
    /// the highest chapter number its text prints ("I, II — ...", "III — ...", "I–III — ...").
    /// </summary>
    internal static bool SagaIsOneChapterFromItsLast(ObjectView saga)
    {
        int lore = saga.Counters.FirstOrDefault(counter => counter.Name == "lore")?.Count ?? 0;
        int last = 0;
        foreach (string line in saga.OracleText.Split('\n'))
        {
            int dash = line.IndexOf('—', StringComparison.Ordinal);
            if (dash <= 0)
            {
                continue;
            }

            foreach (string numeral in line[..dash].Split([',', ' ', '–', '-'], StringSplitOptions.RemoveEmptyEntries))
            {
                last = Math.Max(last, numeral switch { "I" => 1, "II" => 2, "III" => 3, "IV" => 4, "V" => 5, "VI" => 6, _ => 0 });
            }
        }

        return last > 0 && lore == last - 1;
    }

    private AnswerPayload May(GameClient client, PlayerView view, MayPromptMessage prompt)
    {
        int life = LifeOf(view, client.Seat);

        bool accept = prompt.Verb switch
        {
            // Assist (CR 702.132a): this seat pays for its own spells rather than choosing another player to help.
            "Assist" => false,

            // Payments of life - a shockland's, ward's, Master of Death's - are the payment prompt's (WorthPaying).
            "DiscardUnlessCountered" or "DiscardAtRandomUnlessCountered" => client.MyHand().Count > prompt.Amount,

            // Ward—Sacrifice N permanents (Emrakul, the Exigent Doom; Ulamog, the Defiler): a price paid only out of a wide
            // board, which still holds four permanents afterwards.
            "SacrificeUnlessCountered" => client.BattlefieldOf(client.Seat).Count() - prompt.Amount >= 4,

            // Ward—Sacrifice a Food (Ygra, Eater of All): paid when the cheapest Food this seat controls, the one the
            // sacrifice choice would give up, is worth less than the opposing permanents its spell or ability aims at.
            "SacrificeFoodUnlessCountered" => CheapestFood(client) is double food && food < AimedAt(client, prompt.Source),

            // Ad Nauseam: the next card's price is unknown, so keep going only while one more costly card cannot
            // kill — life loss is not a payment and nothing stops a player revealing past zero.
            "RevealAgain" => life >= 10,

            // Ill-Timed Explosion: damage to every creature, this seat's own included, for the price of two cards. Worth it only
            // when the damage the discards would make (see SweepDiscards) kills more creatures across the table than this seat's.
            "DiscardForSweep" => SweepPays(client, view),

            // Pia's Revolution: three damage to keep an artifact out of its owner's hand, while it cannot get close.
            "TakeDamageInsteadOfReturn" => life - prompt.Amount >= 10,

            // Scavenger's Talent, level 3: three other nonland permanents given up for a creature card back from the graveyard.
            // Worth it when the best creature card there is worth more on the battlefield than the three cheapest permanents
            // that could be given up put together.
            "SacrificeThreeToReturn" => BestDead(client, client.Seat)
                > client.BattlefieldOf(client.Seat)
                    .Where(permanent => permanent.Handle != prompt.Source && !IsLand(permanent))
                    .Select(_model.Worth).Order().Take(3).Sum(),

            // Gift (CR 702.174a): what the opponent is given is a price for what the spell does more. A Food or a tapped 1/1
            // Fish is small; a card, a Treasure, an 8/8 Octopus or an extra turn is worth more to them than any spell's
            // better mode is to this seat, so those are never promised.
            "PromiseGiftFood" or "PromiseGiftTappedFish" => true,
            "PromiseGiftCard" or "PromiseGiftTreasure" or "PromiseGiftOctopus" or "PromiseGiftExtraTurn" => false,

            // Garnet, Princess of Alexandria: a lore counter off a Saga is a +1/+1 counter on Garnet and, while the Saga stays, a
            // chapter to run again. It costs the Saga a turn on the way to its final chapter, so it is taken off every Saga
            // except one a single counter short of that chapter, whose payoff it would delay.
            "RemoveLoreCounter" => !(client.Object(prompt.Source) is { } saga && SagaIsOneChapterFromItsLast(saga)),

            // Each of these is a way to lose to itself, or to hand an opponent something.
            "Sacrifice" or "HaveControllerDraw" or "PutIntoGraveyard" or "PutOnBottom"
                or "ChooseOddOrEven" or "ProtectFromColorless" => false,

            // Chain Lightning's copy, when there is something for it to hit — and only a few times a turn, and only
            // of a spell that removes something. Two seats that always copy Chain of Smog pass it back and forth for
            // ever.
            "CopySpell" => _copies < CopiesPerTurn
                && client.Object(prompt.Source) is { } copied
                && CardReading.Effect(CardReading.Text(copied).Split('.')[0]) is { Kind: EffectKind.Damage or EffectKind.Remove } copy
                && Pick(client, view, copy, Guess(client, view, CardReading.Text(copied).Split('.')[0]), 1, optional: true).Worth > Policy("margin")
                && ++_copies > 0,

            // Dredge N (CR 702.52a): the card back for N cards milled in place of a draw. Taken while the library keeps at
            // least ten cards afterwards, so the mill never brings an empty library, and losing to it, close.
            "Dredge" => (Summary(view, client.Seat)?.LibraryCount ?? 0) - prompt.Amount >= 10,

            // Obeka (CR 724.1): asked of the player whose turn it is, so ending it gives up the rest of this seat's own turn.
            // Worth it only to stop an opponent's spell or ability waiting beneath the resolving one, when none of this
            // seat's own would be exiled with it.
            "EndTurn" => client.Zone(ZoneKindView.Stack, 0) is { Count: > 1 } stack
                && stack.Take(stack.Count - 1).All(item => !Mine(client, item)),

            // "You may choose not to untap this creature" (Old Man of the Sea): kept tapped while it holds what it took.
            "UntapInUntapStep" => UntapsInUntapStep(client, prompt),

            // Iroh, Tea Master: the permanent given to an opponent stays with them, so only a token that is no creature (a Food, a
            // Treasure, a Clue) is worth giving for an Ally with a counter for each permanent this seat owns that they control.
            // The prompt's source is the permanent; the targets are those of its resolving ability on the stack, which the view
            // names for its source card.
            "GiveControl" => client.Object(prompt.Source) is { } iroh
                && client.Zone(ZoneKindView.Stack, 0)
                    .Where(item => item.Kind == ObjectKindView.AbilityOnStack && item.Controller == client.Seat && item.Name == iroh.Name)
                    .SelectMany(item => item.Targets)
                    .Select(target => client.Object(target.Object))
                    .OfType<ObjectView>()
                    .Any(given => given.Kind == ObjectKindView.Token && !IsCreature(given)),

            // Everything else a card says its controller "may" do is something a card's controller wants done: a
            // search, a draw, a free spell, an untap.
            _ => true,
        };

        return new MayChosenAnswer { Accept = accept };
    }

    /// <summary>
    /// Which replacement effect applies first (CR 616.1). Only a gain of this seat's life is ordered on purpose: an effect
    /// that turns it into a loss goes first, before anything makes the gain (and so the loss) bigger, and one that adds to
    /// it goes before one that multiplies it. Anything else, and two effects on a loss, are as good in any order to a seat
    /// that does not know what they are. A turn that would begin (CR 614.10) is offered with the effect that must skip it
    /// first: taking that one keeps a "you may skip that turn" (Time Vault) for a later turn, so the default is the choice.
    /// Counters put on this seat's own permanent (it is the one asked, CR 616.1) are made as many as they can be: an effect
    /// that adds one (Hardened Scales' "that many plus one") goes before one that doubles them (Doubling Season), so one
    /// counter becomes four rather than three. An event with -1/-1 counters in it is left to the default: only effects that
    /// multiply apply to those today, so their order changes nothing, and with +1/+1 counters beside them the seat does not
    /// weigh one kind against the other. Damage to this seat
    /// or its permanent is made as little as it can be: a prevention effect goes before one that doubles or adds to it
    /// (Furnace of Rath's ruling: 5 damage, 4 prevented, then doubled, is 2 rather than 6). A card this seat would draw
    /// (CR 121.6) goes to the effect that does most for it: one that wins the game first (Laboratory Maniac); then one that
    /// draws two instead (Thought Reflection), so that each of the two meets the others; then one that puts a card into
    /// its hand instead (dredge, Underrealm Lich, Abundance); then the rest, and last one that discards (Chains of
    /// Mephistopheles). An effect that hands the draw to an opponent (Notion Thief, Hullbreacher's Treasure) goes before
    /// the effects that would make it two, and after those that would keep the card.
    /// </summary>
    private static AnswerPayload Replacement(ChooseReplacementPromptMessage prompt)
    {
        if (prompt.OccurrenceOrder) return prompt.Default();
        if (prompt.Event == "You would draw a card" && prompt.Hints.Count > 0)
        {
            bool keeps = prompt.Hints.Any(hint => hint.Contains("hand instead", StringComparison.Ordinal));
            int Rank(string hint) =>
                hint.Contains("wins the game", StringComparison.Ordinal) ? 0
                : hint.Contains("skips the draw", StringComparison.Ordinal) || hint.Contains("Treasure", StringComparison.Ordinal) ? (keeps ? 5 : 1)
                : hint.Contains("draws two", StringComparison.Ordinal) ? 2
                : hint.Contains("hand instead", StringComparison.Ordinal) ? 3
                : hint.Contains("discards", StringComparison.Ordinal) ? 6
                : 4;
            int best = Enumerable.Range(0, prompt.Hints.Count).MinBy(i => Rank(prompt.Hints[i]));
            return new ReplacementChosenAnswer { Index = best };
        }

        if (prompt.Event.Contains(" would be put on ", StringComparison.Ordinal))
        {
            if (prompt.Event.Contains("-1/-1", StringComparison.Ordinal))
            {
                return prompt.Default();
            }

            int adds = prompt.Hints.FindIndex(hint => hint.Contains("plus", StringComparison.Ordinal) || hint.Contains("one more", StringComparison.Ordinal));
            return adds < 0 ? prompt.Default() : new ReplacementChosenAnswer { Index = adds };
        }

        if (prompt.Event.Contains(" damage from ", StringComparison.Ordinal))
        {
            int prevents = prompt.Hints.FindIndex(hint => hint.Contains("prevent", StringComparison.OrdinalIgnoreCase));
            return prevents < 0 ? prompt.Default() : new ReplacementChosenAnswer { Index = prevents };
        }

        if (!prompt.Event.StartsWith("You would gain", StringComparison.Ordinal))
        {
            return prompt.Default();
        }

        int first = prompt.Hints.FindIndex(hint => hint.Contains("lose", StringComparison.Ordinal));
        if (first < 0)
        {
            first = prompt.Hints.FindIndex(hint => hint.Contains("plus", StringComparison.Ordinal));
        }

        return first < 0 ? prompt.Default() : new ReplacementChosenAnswer { Index = first };
    }

    private static AnswerPayload Player(GameClient client, ChoosePlayerPromptMessage prompt)
    {
        if (prompt.Candidates.Count == 0)
        {
            return prompt.Default();
        }

        bool good = prompt.Verb is "Draw" or "GainLife" or "Scry" or "Surveil" or "CreateTreasure" or "Untap";
        int seat = good && prompt.Candidates.Contains(client.Seat)
            ? client.Seat
            : prompt.Candidates.FirstOrDefault(candidate => candidate != client.Seat, prompt.Candidates[0]);

        return new PlayerChosenAnswer { Player = seat };
    }

    /// <summary>
    /// A color: for protection, that of whatever is being protected from; for mana, one its hand asks for; for a color that
    /// benefits it ("creatures you control of the chosen color get +1/+0"), the one most of its own creatures and spells in
    /// hand are; and otherwise the one its opponents have most of.
    /// </summary>
    private AnswerPayload Colors(GameClient client, PlayerView view, ChooseColorsPromptMessage prompt)
    {
        if (prompt.Choices.Count == 0)
        {
            return prompt.Default();
        }

        if (TextChangeColor(client, view, prompt) is ColorView word)
        {
            return new ColorsChosenAnswer { Colors = [word] };
        }

        IEnumerable<ColorView> theirs = view.Objects
            .Where(card => card.Zone is ZoneKindView.Battlefield or ZoneKindView.Stack && !Mine(client, card))
            .SelectMany(card => card.Colors);

        IEnumerable<ColorView> hand = client.MyHand().Where(card => !IsLand(card)).SelectMany(card => card.Colors);
        IEnumerable<ColorView> ours = view.Objects
            .Where(card => card.Zone == ZoneKindView.Battlefield && Mine(client, card) && IsCreature(card))
            .SelectMany(card => card.Colors);

        IEnumerable<ColorView> source = prompt.Verb switch
        {
            "AddMana" => hand,
            "Benefit" => ours.Concat(hand),
            _ => _threat.Count > 0 ? _threat : theirs,
        };

        List<ColorView> ranked = [.. source
            .Where(prompt.Choices.Contains)
            .GroupBy(color => color)
            .OrderByDescending(group => group.Count())
            .Select(group => group.Key)];

        ranked.AddRange(prompt.Choices.Where(color => !ranked.Contains(color)));
        _threat = [];

        return new ColorsChosenAnswer
        {
            Colors = prompt.MayRepeat ? [.. Enumerable.Repeat(ranked[0], prompt.Count)] : [.. ranked.Take(prompt.Count)],
        };
    }
}
