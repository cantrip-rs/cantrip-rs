using Cantrip.Protocol;

namespace Cantrip.Projection;

/// <summary>
/// A game event as a line of the log, worded for one seat: "You draw [Island]." to the player who drew it, "Bot 2
/// draws a card." to everyone else.
/// <para>
/// Written from the same projected view and events the seat is sent, at the moment they are sent, so a line never
/// names more than the seat was entitled to see then, and names every card it could: a card destroyed three turns ago
/// is still named in the line written when it died, because the line was written while the view still had it.
/// </para>
/// </summary>
public static class LogWriter
{
    /// <summary>One written line: the words, the objects it names in the order it names them, and what it is about.</summary>
    public sealed record Line(string Text, IReadOnlyList<LogReferenceView> References, LogEntryKind Kind);

    /// <summary>The line an event makes for a seat looking at a view, or null for an event that is not a line of its own.</summary>
    public static Line? Write(int seat, PlayerView view, GameEventView entry)
    {
        ArgumentNullException.ThrowIfNull(view);
        ArgumentNullException.ThrowIfNull(entry);

        var reader = new Reader(seat, view);
        string? text = reader.Describe(entry);

        return text is null
            ? null
            : new Line(text, reader.Named, entry switch
            {
                PlayerLostEvent => LogEntryKind.PlayerLost,
                GameEndedEvent { Outcome: GameOutcomeView.Draw } => LogEntryKind.GameDrawn,
                _ => LogEntryKind.Event,
            });
    }

    /// <summary>One seat's reading of one view, collecting the objects a line names as it names them.</summary>
    private sealed class Reader(int seat, PlayerView view)
    {
        private readonly Dictionary<ViewHandle, ObjectView> _objects = Index(view);

        public List<LogReferenceView> Named { get; } = [];

        private static Dictionary<ViewHandle, ObjectView> Index(PlayerView view)
        {
            Dictionary<ViewHandle, ObjectView> byHandle = [];
            foreach (ObjectView item in view.Objects)
            {
                byHandle.TryAdd(item.Handle, item);
            }

            return byHandle;
        }

        private ObjectView? Object(ViewHandle handle) => _objects.GetValueOrDefault(handle);

        public string? Describe(GameEventView entry) => entry switch
        {
            // The move onto the stack says the same thing as the cast, which says it better and names the targets.
            ObjectMovedEvent { To: ZoneKindView.Stack, Reason: ZoneChangeReasonView.Cast } => null,

            // Untapping is the untap step doing its job to everything at once. Listing each permanent buries the
            // turn's actual events under a wall of them.
            TapChangedEvent { Tapped: false } => null,

            // Tapping lands for mana is the cost of what the player does next, and that is the line worth reading.
            TapChangedEvent tap when Object(tap.Object)?.CardTypes.Contains(CardTypeView.Land) == true => null,

            SpellCastEvent cast => Cast(cast),
            ObjectMovedEvent moved => Moved(moved),

            LifeChangedEvent life =>
                $"{(life.Delta >= 0 ? Does(life.Player, "gain", "gains") : Does(life.Player, "lose", "loses"))}" +
                $" {Math.Abs(life.Delta)} life, now {life.NewTotal}.",

            DamageMarkedEvent damage => $"{Card(damage.Object)} is dealt {damage.Amount} damage.",

            TransformedEvent transformed => $"{Card(transformed.Object, transformed.From)} transforms into {Card(transformed.Object, transformed.To)}.",

            // Named after whoever controls it, which the view still carries. A permanent that has already left the
            // view has no controller to name, so it falls back to the passive form.
            TapChangedEvent tap => Object(tap.Object) is { } permanent
                ? $"{Does(permanent.Controller, "tap", "taps")} {Card(tap.Object)}."
                : $"{Card(tap.Object)} is tapped.",

            SpellFizzledEvent fizzled => $"{Card(fizzled.Spell)} does not resolve; none of its targets are legal.",

            LibraryShuffledEvent shuffled => $"{Does(shuffled.Player, "shuffle", "shuffles")}.",

            StartingPlayerChosenEvent { Chooser: var chooser, Player: var first } when chooser == first =>
                $"{Does(chooser, "choose", "chooses")} to play first.",

            // In a duel the only other choice is the opponent, which is choosing to draw first.
            StartingPlayerChosenEvent starting when view.Players.Count == 2 =>
                $"{Does(starting.Chooser, "choose", "chooses")} to draw first.",

            StartingPlayerChosenEvent starting =>
                $"{Does(starting.Chooser, "choose", "chooses")} {(starting.Player == seat ? "you" : NameOf(starting.Player))} to play first.",

            MulliganTakenEvent mulligan => $"{Does(mulligan.Player, "take", "takes")} a mulligan.",

            HandKeptEvent kept => $"{Does(kept.Player, "keep", "keeps")} {kept.Cards} card{(kept.Cards == 1 ? string.Empty : "s")}.",

            DieRolledEvent rolled => $"{Does(rolled.Player, "roll", "rolls")} {rolled.Result} on a d{rolled.Sides}.",

            CoinFlippedEvent flipped => flipped.Called
                ? $"{Does(flipped.Player, "flip", "flips")} a coin: {(flipped.Won ? "won" : "lost")} the flip."
                : $"{Does(flipped.Player, "flip", "flips")} a coin: it came up {(flipped.Heads ? "heads" : "tails")}.",

            // CR 724.1: the stack is exiled (the lines after this one) and the turn goes straight to its cleanup step.
            TurnEndedEvent ended => $"{(ended.Player == seat ? "Your" : $"{NameOf(ended.Player)}'s")} turn ends.",

            // CR 701.22, 701.25: told once the scry or surveil is done, so it follows the cards a surveil put away.
            ScriedEvent scried => $"{Does(scried.Player, "scry", "scries")} {scried.Count}.",

            SurveilledEvent surveilled => $"{Does(surveilled.Player, "surveil", "surveils")} {surveilled.Count}.",

            RandomPlayerChosenEvent chosen =>
                $"{Does(chosen.Controller, "choose", "chooses")} {(chosen.SelectedPlayer == seat ? "you" : NameOf(chosen.SelectedPlayer))} at random" +
                (chosen.Source.IsNone ? "." : $" for {Card(chosen.Source)}."),

            AbilityActivatedEvent activated => Activated(activated),

            TokenCreatedEvent token => $"{Does(token.Controller, "create", "creates")} a {Card(token.Object, token.Name)} token.",

            CountersPutEvent counters when counters.Count < 0 =>
                $"{Card(counters.Object)} loses {-counters.Count} {counters.Name} counter{(counters.Count == -1 ? string.Empty : "s")}.",

            CountersPutEvent counters =>
                $"{Card(counters.Object)} gets {counters.Count} {counters.Name} counter{(counters.Count == 1 ? string.Empty : "s")}.",

            ExtraTurnAddedEvent extra =>
                $"{Does(extra.Player, "take", "takes")} {(extra.Count == 1 ? "an extra turn" : $"{extra.Count} extra turns")} after this one.",

            ExtraPhaseAddedEvent phase => $"There is an additional {PhaseCaption(phase.Phase)} phase after this one.",

            // No player to name: day and night is the game's own designation (CR 731.1), so the line reads as the
            // rules word it.
            DayNightBecameEvent { Designation: DayNightView.Day } => "It becomes day.",

            DayNightBecameEvent { Designation: DayNightView.Night } => "It becomes night.",

            MonarchBecameEvent monarch => $"{Does(monarch.Player, "become", "becomes")} the monarch.",

            PlayerLostEvent lost => $"{Does(lost.Player, "lose", "loses")} the game: {Why(lost)}",

            GameEndedEvent ended => Ended(ended),

            // A shuffle instead is reported by the shuffle itself.
            LibraryArrangedEvent { Shuffled: true } => null,

            LibraryArrangedEvent arranged => (arranged.OnTop, arranged.OnBottom) switch
            {
                (0, 0) => null,
                (_, 0) => $"{Does(arranged.Player, "put", "puts")} {Cards(arranged.OnTop)} back on top.",
                (0, _) => $"{Does(arranged.Player, "put", "puts")} {Cards(arranged.OnBottom)} on the bottom.",
                _ => $"{Does(arranged.Player, "put", "puts")} {Cards(arranged.OnTop)} on top and {Cards(arranged.OnBottom)} on the bottom.",
            },

            // The one who looked reads what they saw; anyone else, only how many cards were looked at.
            CardsLookedAtEvent looked => looked.Cards.Count == 0
                ? null
                : looked.Player == seat
                    ? $"{Does(looked.Player, "look", "looks")} at {string.Join(" and ", looked.Cards.Select(Card))}."
                    : $"{Does(looked.Player, "look", "looks")} at {(looked.Cards.Count == 1 ? "a card" : Cards(looked.Cards.Count))}.",

            CardsRevealedEvent revealed => revealed.Cards.Count == 0
                ? null
                : $"{Does(revealed.Player, "reveal", "reveals")} {string.Join(", ", revealed.Cards.Select(Shown))}.",

            _ => null,
        };

        private string Cast(SpellCastEvent cast)
        {
            string opening = $"{Does(cast.Controller, "cast", "casts")} {Card(cast.Spell)}" +
                (cast.X is { } x ? $" with {{X}} = {x}" : string.Empty) +
                (cast.Buyback
                    ? " with buyback"
                    : cast.Kicked switch { 0 => string.Empty, 1 => " kicked", var times => $" kicked {times} times" }) +
                (cast.Modes.Count > 0 ? $" choosing \"{string.Join("\" and \"", cast.Modes.Select(mode => mode.TrimEnd('.')))}\"" : string.Empty) +
                (cast.GiftPromisedTo is { } giftee ? $", promising a gift to {(giftee == seat ? "you" : NameOf(giftee))}" : string.Empty);

            if (cast.Targets.Count == 0)
            {
                return $"{opening}.";
            }

            List<string> targets = [.. cast.Targets.Select(target => target.IsPlayer ? Subject(target.Player) : Card(target.Object))];
            return $"{opening} targeting {string.Join(" and ", targets)}.";
        }

        /// <summary>
        /// A zone change as the reason for it rather than as the movement: "dies", "is countered" and "is exiled" all
        /// look identical as a movement from the battlefield or stack to a graveyard, and the server sends the reason
        /// precisely so it never has to be guessed.
        /// </summary>
        private string Moved(ObjectMovedEvent moved)
        {
            string card = Card(moved.Object);

            // A shared zone has no owner, so whoever acted is named by whichever side of the move was somebody's. A land
            // played from exile is neither side's, but only the active player plays lands (CR 305.1).
            int actor = moved.FromOwner != 0 ? moved.FromOwner
                : moved.ToOwner != 0 ? moved.ToOwner
                : moved.Reason == ZoneChangeReasonView.Play ? view.ActivePlayer
                : 0;
            string their = actor == seat ? "your" : "their";

            return moved.Reason switch
            {
                ZoneChangeReasonView.Draw => $"{Does(actor, "draw", "draws")} {card}.",
                ZoneChangeReasonView.Play => $"{Does(actor, "play", "plays")} {card}.",
                ZoneChangeReasonView.Discard => $"{Does(actor, "discard", "discards")} {card}.",

                ZoneChangeReasonView.Searched =>
                    $"{Does(actor, "search", "searches")} {their} library and {(actor == seat ? "take" : "takes")} {card}.",

                // A card brought into the game from outside it (CR 400.11b): from its owner's sideboard, which only they see.
                ZoneChangeReasonView.PutIntoHand when moved.From == ZoneKindView.Sideboard =>
                    $"{Does(actor, "put", "puts")} {card} into {their} hand from outside the game.",
                ZoneChangeReasonView.PutIntoHand => $"{Does(actor, "put", "puts")} {card} into {their} hand.",

                ZoneChangeReasonView.Milled => $"{Does(actor, "mill", "mills")} {card}.",
                ZoneChangeReasonView.Surveilled =>
                    $"{Does(actor, "surveil", "surveils")} and {(actor == seat ? "put" : "puts")} {card} into {their} graveyard.",
                ZoneChangeReasonView.Sacrificed => $"{Does(actor, "sacrifice", "sacrifices")} {card}.",
                ZoneChangeReasonView.Countered => $"{card} is countered.",
                ZoneChangeReasonView.Exiled => $"{card} is exiled.",

                // "Returned" is the reason every "put this onto the battlefield" moves under, and a card coming out of
                // a hand was never there to return to — Sneak Attack's creature, Gemstone Caverns beginning the game
                // there (CR 103.6a). Read off the zone it left rather than given a reason of its own.
                ZoneChangeReasonView.Returned when moved is { From: ZoneKindView.Hand, To: ZoneKindView.Battlefield } =>
                    $"{Does(actor, "put", "puts")} {card} onto the battlefield.",

                ZoneChangeReasonView.Returned => $"{card} returns to {Zone(moved.To)}.",

                ZoneChangeReasonView.PutOnLibrary => $"{Does(actor, "put", "puts")} {card} on top of {their} library.",

                ZoneChangeReasonView.ShuffledIn => $"{Does(actor, "shuffle", "shuffles")} {card} into {their} library.",

                // CR 701.40a: the manifested card goes onto the battlefield face down, and manifest dread's other card
                // into the graveyard (CR 701.62a).
                ZoneChangeReasonView.Manifested => moved.To == ZoneKindView.Battlefield
                    ? $"{Does(actor, "manifest", "manifests")} a card face down."
                    : $"{Does(actor, "put", "puts")} {card} into {their} graveyard, not manifesting it.",

                // CR 700.4: a creature going from the battlefield to a graveyard dies, however it got there. Anything
                // else that is destroyed is just destroyed.
                ZoneChangeReasonView.Destroyed or ZoneChangeReasonView.StateBasedAction =>
                    moved is { From: ZoneKindView.Battlefield, To: ZoneKindView.Graveyard }
                        ? Object(moved.Object)?.CardTypes.Contains(CardTypeView.Creature) == false
                            ? $"{card} is destroyed."
                            : $"{card} dies."
                        : $"{card} is put into {Zone(moved.To)}.",

                ZoneChangeReasonView.Resolve => moved.To == ZoneKindView.Battlefield
                    ? $"{card} enters the battlefield."
                    : $"{card} resolves.",

                _ => $"{card} moves to {Zone(moved.To)}.",
            };
        }

        private string Activated(AbilityActivatedEvent activated)
        {
            // Named by its source, as a player would say it: the ability itself is on the stack to be read, and its
            // full text with the cost in front makes for a line nobody reads.
            string opening = (activated.Triggered, activated.Source.IsNone) switch
            {
                // A trigger is nobody's doing, so it is named after what triggered rather than after a player.
                (true, true) => activated.Text.TrimEnd('.'),
                (true, false) => $"{Card(activated.Source)} triggers",
                (false, true) => $"{Does(activated.Controller, "activate", "activates")} {activated.Text.TrimEnd('.')}",
                (false, false) => $"{Does(activated.Controller, "activate", "activates")} {Card(activated.Source)}",
            };

            // A modal ability's modes are announced with it (CR 603.3c, 700.2), worded as a modal spell's are.
            if (activated.Modes.Count > 0)
            {
                opening += $" choosing \"{string.Join("\" and \"", activated.Modes.Select(mode => mode.TrimEnd('.')))}\"";
            }

            if (activated.Targets.Count == 0)
            {
                return $"{opening}.";
            }

            List<string> targets = [.. activated.Targets.Select(target => target.IsPlayer ? NameOf(target.Player) : Card(target.Object))];
            return $"{opening}, targeting {string.Join(" and ", targets)}.";
        }

        /// <summary>
        /// A card by name in brackets, so a name never reads as part of the sentence around it, and noted as one the
        /// line names. "a card" is what the seat reads when it is not entitled to know which one — an opponent
        /// drawing, for instance — and it is deliberately neither bracketed nor noted.
        /// </summary>
        private string Card(ViewHandle handle)
        {
            if (handle.IsNone || Object(handle) is not { Known: true, Name.Length: > 0 } card)
            {
                return "a card";
            }

            Named.Add(new LogReferenceView { Name = card.Name, Object = handle, Card = card });
            return $"[{card.Name}]";
        }

        /// <summary>
        /// A card a player reveals, by name: one still face down - a face-down permanent or spell revealed at the end of the
        /// game, or as its owner leaves it (CR 708.9) - by what it really is, which the seat may now look at, as its own name
        /// is the face-down one's, none.
        /// </summary>
        private string Shown(ViewHandle handle)
        {
            if (!handle.IsNone && Object(handle) is { FaceDown: true, Known: true, FaceDownName.Length: > 0 } card)
            {
                return Card(handle, card.FaceDownName);
            }

            return Card(handle);
        }

        /// <summary>
        /// An object by a name the event gives it rather than its own - the face it had before it transformed, a token
        /// as it was made - bracketed and noted like <see cref="Card(ViewHandle)"/> while the view has it to read.
        /// </summary>
        private string Card(ViewHandle handle, string name)
        {
            if (handle.IsNone || Object(handle) is not { Known: true } card)
            {
                return name;
            }

            Named.Add(new LogReferenceView { Name = name, Object = handle, Card = card });
            return $"[{name}]";
        }

        /// <summary>What took a player out, worded from the loser's side so it reads on from "You lose the game:".</summary>
        private string Why(PlayerLostEvent lost)
        {
            bool mine = lost.Player == seat;

            return lost.Reason switch
            {
                LossReasonView.Life => $"{(mine ? "your" : "their")} life total is {LifeOf(lost.Player)}.",
                LossReasonView.EmptyLibrary => $"{(mine ? "you" : "they")} tried to draw from an empty library.",
                LossReasonView.Poison => "ten poison counters.",
                LossReasonView.Conceded => $"{(mine ? "you" : "they")} conceded.",
                LossReasonView.Effect => "an effect said so.",
                _ => "the rules say so.",
            };
        }

        /// <summary>How the game ended. A team is named by its players; nobody thinks of themselves as a team number.</summary>
        private string Ended(GameEndedEvent ended)
        {
            if (ended.Outcome != GameOutcomeView.TeamWon)
            {
                return ended.DrawReason switch
                {
                    DrawReasonView.MandatoryLoop =>
                        "The game is a draw: it went round a loop in which every player only passed (CR 104.4b).",
                    DrawReasonView.AllLost => "The game is a draw: every player lost at once (CR 104.4a).",
                    DrawReasonView.EventLimit => "The game is a draw: it reached the most events the engine records.",
                    DrawReasonView.Effect => "The game is a draw: an effect says so (CR 104.4c).",
                    _ => "The game is a draw.",
                };
            }

            List<int> winners = [.. view.Players.Where(p => p.Team == ended.WinningTeam).Select(p => p.Seat)];

            if (winners.Contains(seat))
            {
                return winners.Count == 1 ? "You win the game." : "You and your team win the game.";
            }

            return winners.Count == 0
                ? "The game is over."
                : $"{string.Join(" and ", winners.Select(NameOf))} win{(winners.Count == 1 ? "s" : string.Empty)} the game.";
        }

        private int LifeOf(int player) => view.Players.FirstOrDefault(p => p.Seat == player)?.Life ?? 0;

        /// <summary>
        /// A player as the reader thinks of them: themselves as "You", anyone else by name. A log that says "Player 1
        /// draws a card" to Player 1 reads as though it is about somebody else.
        /// </summary>
        private string Subject(int player) => player == seat ? "You" : NameOf(player);

        /// <summary>A subject and its verb, agreeing; English is irregular enough that both forms are given.</summary>
        private string Does(int player, string secondPerson, string thirdPerson) =>
            $"{Subject(player)} {(player == seat ? secondPerson : thirdPerson)}";

        private string NameOf(int player) =>
            player == 0 ? "nobody" : view.Players.FirstOrDefault(p => p.Seat == player)?.Name ?? $"seat {player}";
    }

    private static string Cards(int count) => count == 1 ? "1 card" : $"{count} cards";

    private static string Zone(ZoneKindView kind) => kind switch
    {
        ZoneKindView.Battlefield => "the battlefield",
        ZoneKindView.Graveyard => "a graveyard",
        ZoneKindView.Library => "a library",
        ZoneKindView.Hand => "a hand",
        ZoneKindView.Stack => "the stack",
        ZoneKindView.Exile => "exile",
        ZoneKindView.Sideboard => "outside the game",
        _ => kind.ToString().ToLowerInvariant(),
    };

    /// <summary>Phase names as they are said out loud (CR 500.8's additional phases read "combat"/"main").</summary>
    private static string PhaseCaption(PhaseView phase) => phase switch
    {
        PhaseView.Beginning => "beginning",
        PhaseView.PrecombatMain or PhaseView.PostcombatMain => "main",
        PhaseView.Combat => "combat",
        PhaseView.Ending => "ending",
        _ => "game",
    };
}
