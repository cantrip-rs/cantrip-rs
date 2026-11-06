using Cantrip.Protocol;

namespace Cantrip.Client.Web;

/// <summary>
/// The creatures in combat, in groups: attackers by whom they
/// attack in their controller's creature row, and their blockers straight across the line in the defender's, column
/// for column. What the server has declared and what this player is in the middle of declaring are both shown, so a
/// player sees what they are about to send before they send it.
/// </summary>
public sealed partial class MainPage
{
    /// <summary>Each half's combat groups for this redraw, and the creatures they take out of the rows.</summary>
    private List<BoardRows.CombatGroup> _myCombat = [];
    private List<BoardRows.CombatGroup> _opponentCombat = [];
    private readonly HashSet<ViewHandle> _inCombat = [];

    /// <summary>
    /// A share being divided, worn by the card it goes to: the combat damage each blocker is being sent (CR 510.1c), or
    /// a divided slot's damage or counters (CR 601.2d). The steps on the card make the same change the dock's own would.
    /// </summary>
    private readonly record struct WornShare(int Share, bool Lower, bool Raise, bool Counters, Action<int> Change);

    /// <summary>The shares the cards wear for this redraw, by the card wearing each.</summary>
    private readonly Dictionary<ViewHandle, WornShare> _wornShares = [];

    /// <summary>A step pressed on a card's share: the same change the dock's own rows make.</summary>
    private void OnShareStepped(ViewHandle card, bool raise)
    {
        if (!_wornShares.TryGetValue(card, out WornShare worn) || !(raise ? worn.Raise : worn.Lower))
        {
            return;
        }

        worn.Change(raise ? 1 : -1);
        _needsRedraw = true;
        RedrawNow();
    }

    /// <summary>
    /// Each candidate of a divided slot on the battlefield wears its share (CR 601.2d), so whose creature is getting
    /// what is read off the board, not from a list of names. Down while it has something to take back; up while there
    /// is some of the total left and, where the slot caps its targets, the card is already one or there is room for one.
    /// </summary>
    private void WearDividedShares(GameClient client)
    {
        if (client.Prompt?.Prompt is not ChooseDividedTargetsPromptMessage divide)
        {
            return;
        }

        int assigned = DividedAssigned;
        int targeted = DividedTargeted;
        bool counters = divide.Text.Contains("counter", StringComparison.OrdinalIgnoreCase);

        foreach (TargetMessage candidate in divide.Candidates)
        {
            if (candidate.IsPlayer || client.Object(candidate.Object) is not { Zone: ZoneKindView.Battlefield })
            {
                continue;
            }

            int handle = candidate.Object.Value;
            int share = _dividedShares.GetValueOrDefault(handle);
            bool room = assigned < divide.Total && (share > 0 || divide.MaxTargets == 0 || targeted < divide.MaxTargets);

            _wornShares[candidate.Object] = new WornShare(
                share,
                share > 0,
                room,
                counters,
                delta => _selection.SetDividedShare(candidate, _dividedShares.GetValueOrDefault(handle) + delta));
        }
    }

    private void FillCombat(GameClient client, PlayerView view, float cardWidth)
    {
        _myCombat = [];
        _opponentCombat = [];
        _inCombat.Clear();
        _wornShares.Clear();
        WearDividedShares(client);

        // Groups keep the order their target was first seen in, so a group does not jump as attackers are added.
        List<(string Label, List<(ViewHandle Attacker, List<ViewHandle> Blockers)> Attackers)> groups = [];

        void Add(string label, ViewHandle attacker, List<ViewHandle> blockers)
        {
            int index = groups.FindIndex(g => g.Label == label);
            if (index < 0)
            {
                groups.Add((label, []));
                index = groups.Count - 1;
            }

            groups[index].Attackers.Add((attacker, blockers));
        }

        DeclareBlockersPromptMessage? blocking = client.Prompt?.Prompt as DeclareBlockersPromptMessage;

        foreach (AttackerView attacker in view.Combat.Attackers)
        {
            List<ViewHandle> blockers = [.. view.Combat.Blocks.FirstOrDefault(b => b.Attacker == attacker.Attacker)?.Blockers ?? []];

            // Blocks this player has picked but not yet sent stand with the declared ones.
            if (blocking is not null)
            {
                foreach ((int blocker, int target) in _blocking)
                {
                    if (target == attacker.Attacker.Value)
                    {
                        blockers.Add(new ViewHandle(blocker));
                    }
                }
            }

            Add(AttackTarget(client, attacker), attacker.Attacker, blockers);
        }

        // Attacks being declared, before the server knows about them.
        foreach ((int attacker, AttackChoice choice) in _attacking.OrderBy(pair => pair.Key))
        {
            Add(Describe(client, choice), new ViewHandle(attacker), []);
        }

        foreach ((string label, List<(ViewHandle Attacker, List<ViewHandle> Blockers)> attackers) in groups)
        {
            // Every attacker in a group has the same controller, which decides which half the blockers go in.
            bool mineAttacking = attackers.Any(a => client.Object(a.Attacker)?.Controller == client.Seat);

            // Unblocked attackers stand in a pile with their twins; a blocked one stands alone, because the blockers across
            // from it belong to it and not to the pile - unless a blocker of it blocks another attacker too (a creature that
            // can block more than one, CR 509.1a, or one blocking a band, CR 702.22h): the attackers it blocks then stand side
            // by side in one column across from all their blockers, so each creature is drawn once, and the creature blocking
            // several says which (BlockingAnnotation).
            List<(List<ObjectView> Attackers, List<ObjectView> Blockers)> columns = [];
            Dictionary<string, int> unblocked = [];

            foreach ((ViewHandle attacker, List<ViewHandle> blockerHandles) in attackers)
            {
                if (client.Object(attacker) is not { } creature)
                {
                    continue;
                }

                _inCombat.Add(attacker);
                List<ObjectView> blockers = [.. blockerHandles.Select(client.Object).OfType<ObjectView>()];
                _inCombat.UnionWith(blockers.Select(blocker => blocker.Handle));

                if (blockers.Count == 0)
                {
                    string look = BoardRows.Likeness(creature, PromptLikeness(creature));
                    if (unblocked.TryGetValue(look, out int index))
                    {
                        columns[index].Attackers.Add(creature);
                        continue;
                    }

                    unblocked[look] = columns.Count;
                    columns.Add(([creature], blockers));
                    continue;
                }

                // Every column a blocker of this attacker already stands in becomes one, with this attacker in it.
                List<int> sharing = [.. Enumerable.Range(0, columns.Count)
                    .Where(i => columns[i].Blockers.Exists(standing => blockers.Exists(blocker => blocker.Handle == standing.Handle)))];
                if (sharing.Count == 0)
                {
                    columns.Add(([creature], blockers));
                    continue;
                }

                (List<ObjectView> Attackers, List<ObjectView> Blockers) joined = columns[sharing[0]];
                foreach (int i in sharing.Skip(1))
                {
                    joined.Attackers.AddRange(columns[i].Attackers);
                    joined.Blockers.AddRange(columns[i].Blockers.Where(blocker => !joined.Blockers.Exists(standing => standing.Handle == blocker.Handle)));
                }

                joined.Attackers.Add(creature);
                joined.Blockers.AddRange(blockers.Where(blocker => !joined.Blockers.Exists(standing => standing.Handle == blocker.Handle)));
                for (int i = sharing.Count - 1; i > 0; i--)
                {
                    columns.RemoveAt(sharing[i]);
                }

                // The piles behind a column taken out have moved up one.
                foreach (string look in unblocked.Keys.ToList())
                {
                    unblocked[look] -= sharing.Skip(1).Count(i => i < unblocked[look]);
                }
            }

            // Both columns are given the wider of the two, so an attacker and its blockers share a centre line across
            // the halves. An attacker with no blockers still gets an empty column across from it: "unblocked".
            List<BoardRows.CombatColumn> offence = [];
            List<BoardRows.CombatColumn> defence = [];

            var dividing = client.Prompt?.Prompt as AssignCombatDamagePromptMessage;

            // Each step is offered only where the dock's own would be: down while there is something to take back, up while
            // the creature dividing still has damage left to divide. A creature two divisions reach wears the first's.
            void Wear(DamageDivisionMessage division, ObjectView creature)
            {
                (int Source, int Creature) key = (division.Source.Value, creature.Handle.Value);
                int share = _shares.GetValueOrDefault(key);
                _wornShares.TryAdd(creature.Handle, new WornShare(
                    share,
                    share > 0,
                    Assigned(division) < division.Damage,
                    Counters: false,
                    delta => _shares[key] = Math.Max(0, _shares.GetValueOrDefault(key) + delta)));
            }

            foreach ((List<ObjectView> fighters, List<ObjectView> blockers) in columns)
            {
                // An attacker's damage is worn by its blockers (CR 510.1c), a blocker's by the attackers it blocks (CR 510.1d).
                foreach (ObjectView dealer in fighters.Concat(blockers))
                {
                    if (dividing?.Divisions.FirstOrDefault(candidate => candidate.Source == dealer.Handle) is { } division)
                    {
                        foreach (ObjectView taker in fighters.Concat(blockers).Where(taker => division.Creatures.Contains(taker.Handle)))
                        {
                            Wear(division, taker);
                        }
                    }
                }

                float LineWidth(List<ObjectView> line) =>
                    line.Sum(card => BoardRows.PileWidthOf([card], cardWidth)) + (BoardRows.BlockerGap * Math.Max(0, line.Count - 1));

                bool pile = blockers.Count == 0;
                float width = MathF.Max(pile ? BoardRows.PileWidthOf(fighters, cardWidth) : LineWidth(fighters), LineWidth(blockers));

                offence.Add(new BoardRows.CombatColumn(width, fighters, Pile: pile));
                defence.Add(new BoardRows.CombatColumn(width, blockers, Pile: false));
            }

            (mineAttacking ? _myCombat : _opponentCombat).Add(new BoardRows.CombatGroup($"Attacking {label}", offence));
            (mineAttacking ? _opponentCombat : _myCombat).Add(new BoardRows.CombatGroup(null, defence));
        }
    }

    /// <summary>Whom or what an attacker was sent at, by name (CR 508.1a).</summary>
    private static string AttackTarget(GameClient client, AttackerView attacker) => attacker.TargetKind switch
    {
        AttackTargetKindView.Player => NameOf(client, attacker.TargetPlayer),
        _ => client.Object(attacker.TargetPermanent)?.Name ?? "?",
    };

    /// <summary>Whom or what an attacker is being sent at, by name, before the server knows.</summary>
    private static string Describe(GameClient client, AttackChoice choice) =>
        choice.Permanent == ViewHandle.None
            ? NameOf(client, choice.Player)
            : client.Object(choice.Permanent)?.Name ?? "?";
}
