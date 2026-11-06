using System.Globalization;
using System.Text;
using Cantrip.Protocol;

namespace Cantrip.Client.Llm;

/// <summary>
/// The position, written out for something that reads rather than looks.
/// <para>
/// The same job the web client's board does, to a different medium: everything the server has told this seat it
/// may see, and nothing else. It reads <see cref="PlayerView"/> and nothing but — no engine, no rules, no
/// inference — so a card's power here is the power the layers computed and a hidden zone is a count, exactly as on
/// screen.
/// </para>
/// <para>
/// Objects carry a <c>#n</c> tag taken from their handle. Not because the model answers with one — answers are
/// always a key into a list of choices — but because two Grizzly Bears are otherwise the same words twice, and a
/// question about one of them has to be able to say which.
/// </para>
/// </summary>
public static class GameText
{
    /// <summary>The whole position as this seat may see it: who is where, what is on the board, what is in hand.</summary>
    public static string Position(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (client.View is not { } view)
        {
            return "The game has not started.";
        }

        var text = new StringBuilder();

        text.Append(CultureInfo.InvariantCulture, $"Turn {view.Turn}, {Step(view.Step)}.");
        text.Append(CultureInfo.InvariantCulture, $" It is {Whose(client, view.ActivePlayer)} turn.");

        // Control of a player (CR 723): the controller makes every decision of theirs, with their cards and mana, and sees
        // their hand; the controlled player makes none.
        if (client.ControlledSeat is > 0 and var controlled)
        {
            text.Append(CultureInfo.InvariantCulture,
                $" You control {Seat(client, controlled)} this turn: you make every decision they would make, paying with their cards and mana, and you see their hand. Their permanents stay theirs.");
        }
        else if (client.Me is { ControlledBy: > 0 } me && me.ControlledBy != client.Seat)
        {
            text.Append(CultureInfo.InvariantCulture, $" {Seat(client, me.ControlledBy)} controls you this turn and makes your decisions.");
        }

        if (view.DayNight != DayNightView.Neither)
        {
            text.Append(CultureInfo.InvariantCulture, $" It is {view.DayNight.ToString().ToLowerInvariant()}.");
        }

        text.Append('\n');

        foreach (PlayerSummaryView player in view.Players)
        {
            text.Append('\n').Append(Player(client, player));
        }

        Pile(text, "\nThe stack (top of the stack first, so the first one resolves next)", Stack(client));

        foreach (PlayerSummaryView player in view.Players)
        {
            Pile(
                text,
                $"\n{Owner(client, player.Seat)} battlefield",
                [.. client.BattlefieldShownOf(player.Seat).Select(card => Card(client, card))]);
        }

        Pile(text, $"\n{Owner(client, client.Seat)} hand", [.. client.MyHand().Select(card => Card(client, card))]);

        if (client.ControlledSeat is > 0 and var theirs)
        {
            Pile(text, $"\n{Owner(client, theirs)} hand", [.. client.Zone(ZoneKindView.Hand, theirs).Select(card => Card(client, card))]);
        }

        foreach (PlayerSummaryView player in view.Players)
        {
            Pile(
                text,
                $"\n{Owner(client, player.Seat)} graveyard",
                [.. client.Zone(ZoneKindView.Graveyard, player.Seat).Select(card => Card(client, card, oracle: false))]);

            Pile(
                text,
                $"\n{Owner(client, player.Seat)} exile",
                [.. client.Zone(ZoneKindView.Exile, player.Seat).Select(card => Card(client, card, oracle: false))]);
        }

        Combat(text, client, view);
        return text.ToString();
    }

    /// <summary>One object as a line: what it is, what it is doing, and what it says.</summary>
    public static string Card(GameClient client, ObjectView card, bool oracle = true)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(card);

        var text = new StringBuilder();
        text.Append(Tag(card.Handle)).Append(' ').Append(Name(card));

        if (card.ManaCost.Length > 0)
        {
            text.Append(' ').Append(card.ManaCost);
        }

        if (Types(card) is { Length: > 0 } types)
        {
            text.Append(" | ").Append(types);
        }

        if (card.Power is { } power && card.Toughness is { } toughness)
        {
            text.Append(CultureInfo.InvariantCulture, $" | {power}/{toughness}");

            if (card.DamageMarked > 0)
            {
                text.Append(CultureInfo.InvariantCulture, $" with {card.DamageMarked} damage marked");
            }
        }

        if (State(card) is { Length: > 0 } state)
        {
            text.Append(" | ").Append(state);
        }

        if (card.AttachedTo != ViewHandle.None)
        {
            text.Append(" | attached to ").Append(Named(client, card.AttachedTo));
        }

        if (card.Targets.Count > 0)
        {
            text.Append(" | targeting ").Append(string.Join(", ", card.Targets.Select(target => Aimed(client, target))));
        }

        if (oracle && card.OracleText.Length > 0)
        {
            text.Append(" | \"").Append(card.OracleText.ReplaceLineEndings(" / ")).Append('"');
        }

        if (oracle && card.GrantedAbilities.Count > 0)
        {
            text.Append(" | granted: \"").Append(string.Join(" / ", card.GrantedAbilities.Select(granted => granted.Text)))
                .Append('"');
        }

        if (oracle && card.LostAbilities.Count > 0)
        {
            text.Append(" | lost: \"").Append(string.Join(" / ", card.LostAbilities)).Append('"');
        }

        foreach (PlayPermissionView permission in card.PlayPermissions)
        {
            text.Append(" | ").Append(permission.Text);
        }

        if (oracle && card.OtherFace is { Name.Length: > 0 } back)
        {
            text.Append(" | other face: ").Append(Card(client, back, oracle: true));
        }

        if (oracle && card.PrepareSpell is { Name.Length: > 0 } prepare)
        {
            text.Append(" | prepare spell: ").Append(Card(client, prepare, oracle: true));
        }

        // An Adventure or Omen (CR 715.2, 720.2): a spell the card may be cast as instead of itself, named by its subtype.
        if (oracle && card.InsetSpell is { Name.Length: > 0 } inset)
        {
            text.Append(" | may be cast instead as: ").Append(Card(client, inset, oracle: true));
        }

        return text.ToString();
    }

    /// <summary>One object by name and tag, for a choice or a sentence that refers to it.</summary>
    public static string Named(GameClient client, ViewHandle handle)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (handle == ViewHandle.None)
        {
            return "nothing";
        }

        ObjectView? card = client.Object(handle);
        return card is null ? Tag(handle) : $"{Name(card)} {Tag(handle)}";
    }

    /// <summary>One object by name, tag, and the stats a combat decision turns on.</summary>
    public static string Creature(GameClient client, ViewHandle handle)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (client.Object(handle) is not { } card)
        {
            return Named(client, handle);
        }

        var text = new StringBuilder(Named(client, handle));

        if (card.Power is { } power && card.Toughness is { } toughness)
        {
            text.Append(CultureInfo.InvariantCulture, $" {power}/{toughness}");

            if (card.DamageMarked > 0)
            {
                text.Append(CultureInfo.InvariantCulture, $" ({card.DamageMarked} damage marked)");
            }
        }

        if (State(card) is { Length: > 0 } state)
        {
            text.Append(", ").Append(state);
        }

        if (card.OracleText.Length > 0)
        {
            text.Append(" — \"").Append(card.OracleText.ReplaceLineEndings(" / ")).Append('"');
        }

        return text.ToString();
    }

    /// <summary>Something a spell may be aimed at, whether it is a player or an object.</summary>
    public static string Aimed(GameClient client, TargetMessage target)
    {
        ArgumentNullException.ThrowIfNull(target);
        return target.IsPlayer ? Player(client, target.Player) : Named(client, target.Object);
    }

    /// <summary>The same for something already aimed at, which may have stopped being a legal target.</summary>
    public static string Aimed(GameClient client, SpellTargetView target)
    {
        ArgumentNullException.ThrowIfNull(target);

        string what = target.IsPlayer ? Player(client, target.Player) : Named(client, target.Object);
        string how = target.Text.Length > 0 ? $" ({target.Text})" : string.Empty;
        return target.StillLegal ? what + how : $"{what}{how} [no longer a legal target]";
    }

    /// <summary>
    /// What a seat is called here: "you", or "Opponent".
    /// <para>
    /// Never the player's own name, and that is the whole point. The client calls its human player "You" by
    /// default, so a seat reading its opponent by name was offered "you" and "You" as two separate things to point
    /// a burn spell at — which is how a model came to shoot itself in the face twice in one game. A name carries
    /// nothing a decision needs anyway: what matters about the other side of the table is only that it is the
    /// other side of the table.
    /// </para>
    /// <para>
    /// The seat is added only where there is more than one opponent, which is where it becomes the one thing that
    /// tells them apart.
    /// </para>
    /// </summary>
    public static string Seat(GameClient client, int seat)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (seat == 0)
        {
            return "nobody";
        }

        if (seat == client.Seat)
        {
            return "you";
        }

        return Alone(client) ? "Opponent" : $"Opponent (seat {seat.ToString(CultureInfo.InvariantCulture)})";
    }

    /// <summary>
    /// A seat as a choice names it: "Yourself", or "Opponent".
    /// <para>
    /// Apart from <see cref="Seat"/> only in the self form, because a choice is read on its own — "you" in a list of
    /// things to point a spell at is a word a sentence would have supplied a subject for, and "Yourself" is not.
    /// </para>
    /// </summary>
    public static string Player(GameClient client, int seat)
    {
        ArgumentNullException.ThrowIfNull(client);
        return seat == client.Seat ? "Yourself" : Seat(client, seat);
    }

    /// <summary>The possessive: "your", "Opponent's".</summary>
    public static string Whose(GameClient client, int seat)
    {
        ArgumentNullException.ThrowIfNull(client);
        return seat == client.Seat ? "your" : $"{Seat(client, seat)}'s";
    }

    /// <summary>The same, to start a sentence or a heading with: "Your", "Opponent's".</summary>
    public static string Owner(GameClient client, int seat)
    {
        string whose = Whose(client, seat);
        return whose.Length > 0 ? char.ToUpperInvariant(whose[0]) + whose[1..] : whose;
    }

    /// <summary>Whether this seat has exactly one opponent, so "Opponent" names it without ambiguity.</summary>
    private static bool Alone(GameClient client) => client.View is not { } view || view.Players.Count <= 2;

    /// <summary>
    /// One thing that has happened, as a sentence, or null for one not worth a line.
    /// <para>
    /// Written as the event arrives and never later: a <see cref="ViewHandle"/> only names something while this seat
    /// can still see it, and the server drops it one projection afterwards. A creature that died two turns ago
    /// resolves to nothing, so a log rendered at the end of a game could only say "a card".
    /// </para>
    /// </summary>
    public static string? Line(GameClient client, GameEventView entry)
    {
        ArgumentNullException.ThrowIfNull(client);

        return entry switch
        {
            StepBeganEvent step => $"--- Turn {step.Turn}, {Step(step.Step)} ({Seat(client, step.ActivePlayer)}) ---",

            SpellCastEvent cast =>
                $"{Seat(client, cast.Controller)} cast {Named(client, cast.Spell)}"
                + Aimed(client, cast.Targets)
                + (cast.X is { } x ? $" with X={x}" : string.Empty)
                + (cast.Modes.Count > 0 ? $" [{string.Join("; ", cast.Modes)}]" : string.Empty)
                + (cast.Kicked > 0 ? " (kicked)" : string.Empty),

            AbilityActivatedEvent ability =>
                $"{Seat(client, ability.Controller)} {(ability.Triggered ? "triggered" : "activated")} "
                + $"{Named(client, ability.Source)}: \"{ability.Text}\"{Aimed(client, ability.Targets)}"
                + (ability.Modes.Count > 0 ? $" [{string.Join("; ", ability.Modes)}]" : string.Empty),

            SpellFizzledEvent fizzle =>
                $"{Named(client, fizzle.Spell)} was countered: every target it had was gone.",

            ObjectMovedEvent moved => Moved(client, moved),

            LifeChangedEvent life =>
                $"{Seat(client, life.Player)} {(life.Delta < 0 ? "lost" : "gained")} {Math.Abs(life.Delta)} life "
                + $"(now {life.NewTotal}).",

            DamageMarkedEvent damage => $"{Named(client, damage.Object)} was dealt {damage.Amount} damage.",

            CountersPutEvent counters =>
                $"{Named(client, counters.Object)} got {counters.Count} {counters.Name} counter(s).",

            TokenCreatedEvent token =>
                $"{Seat(client, token.Controller)} created a {token.Name} token.",

            TransformedEvent changed => $"{changed.From} transformed into {changed.To}.",

            MonarchBecameEvent monarch => $"{Seat(client, monarch.Player)} became the monarch.",

            DayNightBecameEvent designation => $"It became {designation.Designation.ToString().ToLowerInvariant()}.",

            CardsLookedAtEvent looked when looked.Cards.Count > 0 =>
                $"{Seat(client, looked.Player)} looked at "
                + string.Join(", ", looked.Cards.Select(card => Named(client, card))) + ".",

            CardsRevealedEvent revealed when revealed.Cards.Count > 0 =>
                $"{Seat(client, revealed.Player)} revealed "
                + string.Join(", ", revealed.Cards.Select(card => Named(client, card))) + ".",

            DieRolledEvent die => $"{Seat(client, die.Player)} rolled a {die.Result} on a d{die.Sides}.",

            ScriedEvent scried => $"{Seat(client, scried.Player)} scried {scried.Count}.",

            SurveilledEvent surveilled => $"{Seat(client, surveilled.Player)} surveilled {surveilled.Count}.",

            CoinFlippedEvent flip => flip.Called
                ? $"{Seat(client, flip.Player)} flipped a coin and {(flip.Won ? "won" : "lost")} the flip."
                : $"{Seat(client, flip.Player)} flipped a coin and it came up {(flip.Heads ? "heads" : "tails")}.",

            TurnEndedEvent ended => (ended.Player == client.Seat ? "Your" : $"{Seat(client, ended.Player)}'s")
                + " turn was ended: the stack was exiled and the turn went straight to its cleanup step.",

            ExtraTurnAddedEvent extra => $"{Seat(client, extra.Player)} takes {extra.Count} extra turn(s).",

            PlayerLostEvent lost => $"{Seat(client, lost.Player)} lost the game ({Loss(lost.Reason)}).",

            GameEndedEvent ended => ended.Outcome == GameOutcomeView.Draw
                ? ended.DrawReason switch
                {
                    DrawReasonView.MandatoryLoop => "The game ended in a draw: it went round a loop in which every player only passed (CR 104.4b).",
                    DrawReasonView.AllLost => "The game ended in a draw: every player lost at once.",
                    DrawReasonView.EventLimit => "The game ended in a draw: it reached the most events the engine records.",
                    _ => "The game ended in a draw.",
                }
                : $"The game is over; team {ended.WinningTeam} won.",

            // Tapping, shuffling and library arrangement are all visible in the position itself, and a line for each
            // would bury the ones that are not.
            _ => null,
        };
    }

    private static string Moved(GameClient client, ObjectMovedEvent moved)
    {
        string what = Named(client, moved.Object);

        return (moved.From, moved.To) switch
        {
            (ZoneKindView.Library, ZoneKindView.Hand) => $"{Owner(client, moved.ToOwner)} drew a card.",
            (ZoneKindView.Stack, ZoneKindView.Battlefield) => $"{what} resolved and entered the battlefield.",
            (ZoneKindView.Stack, ZoneKindView.Graveyard) => $"{what} resolved and went to the graveyard.",
            (ZoneKindView.Battlefield, ZoneKindView.Graveyard) => $"{what} was put into the graveyard.",
            (ZoneKindView.Hand, ZoneKindView.Graveyard) => $"{Owner(client, moved.FromOwner)} discarded {what}.",
            (_, ZoneKindView.Exile) => $"{what} was exiled.",
            (ZoneKindView.Hand, ZoneKindView.Battlefield) => $"{Seat(client, moved.ToOwner)} put {what} onto the battlefield.",
            _ => $"{what} went from {Zone(moved.From)} to {Zone(moved.To)}.",
        };
    }

    private static string Aimed(GameClient client, List<TargetMessage> targets) =>
        targets.Count == 0
            ? string.Empty
            : ", targeting " + string.Join(", ", targets.Select(target => Aimed(client, target)));

    private static string Player(GameClient client, PlayerSummaryView player)
    {
        var text = new StringBuilder();
        text.Append(Player(client, player.Seat));
        text.Append(CultureInfo.InvariantCulture, $": {player.Life} life");
        text.Append(CultureInfo.InvariantCulture, $", {player.HandCount} card(s) in hand");
        text.Append(CultureInfo.InvariantCulture, $", {player.LibraryCount} left in library");

        if (player.Mana.Sum() > 0)
        {
            text.Append(", unspent mana ").Append(Mana(player.Mana));
        }

        // Mana a rule tells apart by the permanent that produced it (CR 106.3): "1 white from Barracks of the Thousand".
        foreach (RestrictedManaView apart in player.RestrictedMana.Where(mana => !mana.Source.IsNone || mana.Snow))
        {
            text.Append(CultureInfo.InvariantCulture,
                $", unspent {(apart.Snow ? "snow-source " : "")}mana {apart.Amount} {apart.Type.ToString().ToLowerInvariant()}{(apart.Source.IsNone ? "" : $" from {GameText.Named(client, apart.Source)}")}");
        }

        if (player.PoisonCounters > 0)
        {
            text.Append(CultureInfo.InvariantCulture, $", {player.PoisonCounters} poison");
        }

        if (player.Energy > 0)
        {
            text.Append(CultureInfo.InvariantCulture, $", {player.Energy} energy");
        }

        foreach (CounterView counter in player.Counters)
        {
            text.Append(CultureInfo.InvariantCulture, $", {counter.Count} {counter.Name} counter(s)");
        }

        foreach (string pending in player.PendingEffects)
        {
            text.Append(", ").Append(pending.TrimEnd('.'));
        }

        if (player.IsMonarch)
        {
            text.Append(", the monarch");
        }

        if (player.HasInitiative)
        {
            text.Append(", has the initiative");
        }

        if (player.HasCitysBlessing)
        {
            text.Append(", has the city's blessing");
        }

        if (player.HasEnduringStory)
        {
            text.Append(", has an enduring story");
        }

        // Speed (CR 702.179): max speed abilities work only at 4 (CR 702.178a).
        if (player.Speed is int speed)
        {
            text.Append(CultureInfo.InvariantCulture, $", speed {speed} of 4");
            if (speed >= 4)
            {
                text.Append(" (max speed)");
            }
        }

        if (player.Dungeon.Length > 0)
        {
            text.Append(CultureInfo.InvariantCulture, $", in {player.Dungeon} at {player.DungeonRoom}");
        }

        if (player.Status != PlayerStatusView.Active)
        {
            text.Append(CultureInfo.InvariantCulture, $", {player.Status.ToString().ToLowerInvariant()}");
        }

        return text.Append('.').ToString();
    }

    private static void Combat(StringBuilder text, GameClient client, PlayerView view)
    {
        if (view.Combat.Attackers.Count == 0)
        {
            return;
        }

        text.Append("\n\nCombat:");

        foreach (AttackerView attacker in view.Combat.Attackers)
        {
            string at = attacker.TargetKind == AttackTargetKindView.Player
                ? Seat(client, attacker.TargetPlayer)
                : Named(client, attacker.TargetPermanent);

            List<ViewHandle> blockers = view.Combat.Blocks
                .Where(block => block.Attacker == attacker.Attacker)
                .SelectMany(block => block.Blockers)
                .ToList();

            text.Append("\n  ").Append(Creature(client, attacker.Attacker)).Append(" is attacking ").Append(at);

            // An attacking band (CR 702.22c, 702.22h): blocked as one.
            if (attacker.Band > 0)
            {
                text.Append(" in band ").Append(attacker.Band);
            }

            text.Append(blockers.Count > 0
                ? ", blocked by " + string.Join(", ", blockers.Select(blocker => Creature(client, blocker)))
                : attacker.WasBlocked ? ", blocked" : ", unblocked");
        }
    }

    private static List<string> Stack(GameClient client)
    {
        // Zone order puts the bottom of the stack first; the top resolves next, so it is read the other way round.
        List<string> lines = [.. client.Zone(ZoneKindView.Stack, 0).Select(card => Card(client, card))];
        lines.Reverse();
        return lines;
    }

    private static void Pile(StringBuilder text, string heading, IReadOnlyList<string> lines)
    {
        if (lines.Count == 0)
        {
            return;
        }

        text.Append('\n').Append(heading).Append(':');

        foreach (string line in lines)
        {
            text.Append("\n  ").Append(line);
        }
    }

    private static string Name(ObjectView card)
    {
        if (card.Name.Length > 0)
        {
            return card.FaceDownName.Length > 0 ? $"{card.Name} (really {card.FaceDownName})" : card.Name;
        }

        return card.FaceDownName.Length > 0 ? $"a face-down card (really {card.FaceDownName})" : "a face-down card";
    }

    private static string Types(ObjectView card)
    {
        IReadOnlyList<string> subtypes = card.ShownSubtypes;
        if (card.CardTypes.Count == 0 && subtypes.Count == 0 && !card.EveryCreatureType)
        {
            return string.Empty;
        }

        string front = string.Join(
            " ",
            card.Supertypes.Select(type => type.ToString()).Concat(card.CardTypes.Select(type => type.ToString())));

        // Every creature type (Changeling) is said in words: its three hundred types would fill the prompt.
        if (card.EveryCreatureType)
        {
            return subtypes.Count > 0 ? $"{front} - {string.Join(" ", subtypes)}, every creature type" : $"{front} - every creature type";
        }

        return subtypes.Count > 0 ? $"{front} - {string.Join(" ", subtypes)}" : front;
    }

    /// <summary>Everything about a permanent that is true right now and is not one of its printed characteristics.</summary>
    private static string State(ObjectView card)
    {
        List<string> parts = [];

        if (card.Tapped)
        {
            parts.Add("tapped");
        }

        if (card.SummoningSick)
        {
            parts.Add("summoning sick, so it cannot attack, and its {T} abilities only where an effect lets them be activated as though it had haste (the actions offered say)");
        }

        foreach (CounterView counter in card.Counters)
        {
            parts.Add(string.Create(CultureInfo.InvariantCulture, $"{counter.Count} {counter.Name} counter(s)"));
        }

        foreach (ChoiceView choice in card.Choices)
        {
            parts.Add($"{choice.Key} chosen: {choice.Value}");
        }

        if (card.Monstrous)
        {
            parts.Add("monstrous");
        }

        if (card.Solved)
        {
            parts.Add("solved");
        }

        if (card.Suspected)
        {
            parts.Add("suspected");
        }

        if (card.PhasedOut)
        {
            parts.Add("phased out: treated as though it doesn't exist until it phases in");
        }

        if (card.ClassLevel > 0)
        {
            parts.Add($"level {card.ClassLevel}");
        }

        if (card.Harnessed)
        {
            parts.Add("harnessed");
        }

        if (card.Exerted)
        {
            parts.Add("exerted");
        }

        if (card.Saddled)
        {
            parts.Add("saddled");
        }

        if (card.Goaded)
        {
            parts.Add("goaded, so it must attack");
        }

        if (card.CantBeCountered && card.Zone == ZoneKindView.Stack)
        {
            parts.Add("can't be countered");
        }

        return string.Join(", ", parts);
    }

    private static string Mana(int[] pool)
    {
        List<string> parts = [];

        for (int type = 0; type < pool.Length; type++)
        {
            if (pool[type] > 0)
            {
                parts.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $"{pool[type]} {((ManaTypeView)type).ToString().ToLowerInvariant()}"));
            }
        }

        return string.Join(", ", parts);
    }

    private static string Tag(ViewHandle handle) =>
        string.Create(CultureInfo.InvariantCulture, $"#{handle.Value}");

    private static string Zone(ZoneKindView zone) => zone switch
    {
        ZoneKindView.Battlefield => "the battlefield",
        ZoneKindView.Stack => "the stack",
        // The cards a player owns outside the game (CR 400.11a): "went from outside the game to a hand".
        ZoneKindView.Sideboard => "outside the game",
        _ => "a " + zone.ToString().ToLowerInvariant(),
    };

    private static string Loss(LossReasonView reason) => reason switch
    {
        LossReasonView.Life => "life at zero",
        LossReasonView.EmptyLibrary => "drew from an empty library",
        LossReasonView.Poison => "ten poison counters",
        LossReasonView.Conceded => "conceded",
        _ => "an effect said so",
    };

    /// <summary>A step, written the way a player says it: "precombat main phase" rather than "PrecombatMain".</summary>
    public static string Step(TurnStepView step) => step switch
    {
        TurnStepView.Untap => "untap step",
        TurnStepView.Upkeep => "upkeep",
        TurnStepView.Draw => "draw step",
        TurnStepView.PrecombatMain => "precombat main phase",
        TurnStepView.BeginCombat => "beginning of combat",
        TurnStepView.DeclareAttackers => "declare attackers step",
        TurnStepView.DeclareBlockers => "declare blockers step",
        TurnStepView.CombatDamage => "combat damage step",
        TurnStepView.EndOfCombat => "end of combat",
        TurnStepView.PostcombatMain => "postcombat main phase",
        TurnStepView.End => "end step",
        _ => "cleanup step",
    };
}
