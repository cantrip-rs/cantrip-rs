using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>
/// Answers prompts for a seat nobody is driving.
/// <para>
/// A function over a <see cref="GameClient"/> rather than a client of its own, because a seat has exactly one
/// mailbox: two clients bound to the same seat would race for its messages and whichever drained first would
/// consume the prompt the other was waiting for.
/// </para>
/// <para>
/// Not an opponent worth playing against, and not meant to be. It exists so the turn structure keeps moving, and
/// it goes through the same client interface a human does, so it cannot accidentally be granted anything a real
/// client would not have.
/// </para>
/// <para>
/// A decision of a player this seat controls (CR 723.5) it makes as that player would (<see cref="GameClient.ActingSeat"/>):
/// their main phase, their creatures and lands, their opponents. Legal either way, and the turn keeps moving.
/// </para>
/// </summary>
public static class AutoPlay
{
    /// <summary>Answers if the client has been asked something. Returns true if it answered.</summary>
    public static bool Act(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (client.IsOver || !client.IsWaitingOnMe)
        {
            return false;
        }

        switch (client.Prompt!.Prompt)
        {
            case DiscardPromptMessage discard:
                client.Discard(discard.Hand.Take(discard.Count));
                return true;

            case PriorityPromptMessage priority:
                ActOnPriority(client, priority);
                return true;

            case PayCostsPromptMessage pay:
                TakeActionAnswer paying = Pay(pay);
                AttackCosts.Answered(client, pay, paying);
                client.Answer(paying);
                return true;

            case UnlessPaysPromptMessage unless:
                // Pays, but not Lim-Dûl's Vault's "as many times as you choose, you may pay 1 life": a seat that always
                // pays buys look after look until it has no life left and dies to it (CR 704.5a).
                client.Answer(unless.Verb == "LookAgain" ? Answer(unless.Options.First(option => option.Kind == ActionKindView.DeclineCost)) : PayUnless(unless));
                return true;

            case ChooseCardsPromptMessage { Purpose: CardChoicePurposeView.Band }:
                // Attackers banded together (CR 702.22c): none, so each is blocked on its own.
                client.Answer(new CardsChosenAnswer());
                return true;

            case ChooseCardsPromptMessage choose:
                // The first eligible cards. Which card hurts most to lose is strategy this does not have.
                client.Answer(new CardsChosenAnswer { Cards = [.. choose.Verdict.Addable.Take(choose.Count)] });
                return true;

            case DistributeCardsPromptMessage distribute:
                client.Answer(Distribute(client, distribute));
                return true;

            case DeclareAttackersPromptMessage attackers:
                // Attacks with everything it can, and can pay for (CR 508.1h). Terrible play, but it makes games finish by
                // damage rather than by running out of cards, which is what makes a self-played game a useful smoke test.
                client.Answer(new AttackersDeclaredAnswer
                {
                    Attacks = AttackCosts.Within(
                        attackers,
                        [
                            // Each where the engine's own declaration sends it (CR 508.1d), and any other at the first thing the
                            // host's verdict offers it (CR 508.1c): a preference, judged by the host as GameClient.Answer settles it.
                            .. attackers.Eligible.Select(creature =>
                                attackers.SuggestedFor(creature)
                                    ?? attackers.OfferedFor(creature).FirstOrDefault()
                                    ?? attackers.AttacksFor(creature)[0]),
                        ],
                        AttackCosts.Mana(client, attackers)),
                });

                return true;

            case DeclareBlockersPromptMessage blockers:
                client.Answer(Block(client, blockers));
                return true;

            case SearchLibraryPromptMessage search:
                // Takes the first cards it is allowed to. Which card is worth having is strategy this does not have,
                // and taking the most allowed is legal whether or not the search was optional.
                client.Answer(new CardsChosenAnswer { Cards = [.. search.Verdict.Addable.Take(search.Count)] });
                return true;

            case ChooseTargetPromptMessage target:
                client.Answer(new TargetsChosenAnswer { Targets = ChooseTarget(client, target) });
                return true;

            case ChooseModesPromptMessage modes:
                // The first modes on offer that it could pay for: which is worth more is strategy this does not have, but a
                // spree mode it cannot afford would be chosen, given up on while paying and chosen again for ever.
                // Monocolored hybrid symbols ({2/W}, CR 601.2b): those the server advises paying with {2} for the mana to cover it.
                client.Answer(new ModesChosenAnswer
                {
                    Modes = modes.MayRepeat ? [.. Enumerable.Repeat(modes.Eligible[0], modes.Count)]
                        : modes.AdditionalCost && modes.Count == 0 && modes.Modes.Count > 0
                            && modes.Modes.TrueForAll(mode => mode.StartsWith("Pay {2} for {2/", StringComparison.Ordinal)) ? [.. modes.Affordable]
                        : [.. modes.Preferred.Take(modes.Count)],
                });

                return true;

            case MayPromptMessage may:
                // Does it: a search it may make is one it would rather make, and saying yes exercises more of the game.
                // Not a Chain of Smog copy, though: two seats that always copy it pass it back and forth for ever.
                // Nor Ad Nauseam's "you may repeat this process", which reveals until it dies to it.
                // Nor assist's "choose another player to help pay" (CR 702.132a): this seat pays for its own spells.
                client.Answer(new MayChosenAnswer { Accept = may.Verb is not ("CopySpell" or "RevealAgain" or "Assist") });
                return true;

            case AssignCombatDamagePromptMessage assign:
                // The engine's own division — lethal to each creature in turn, the rest through with trample; for an
                // opponent's creature banding gives this seat (CR 702.22j-k), all of it on the creature it hurts least — is
                // always legal and as good as a bot needs.
                client.Answer(assign.Default());
                return true;

            default:
                // A question this build has no opinion about — a prompt added to the server since, most likely.
                // The prompt itself knows the simplest legal answer, so an unfamiliar one costs a bad decision
                // rather than a game that quietly stops with nobody able to move.
                client.Answer(client.Prompt!.Prompt.Default());
                return true;
        }
    }

    /// <summary>
    /// Blocks in two passes: first any single blocker that survives and kills, then a gang block on whatever is
    /// still coming through. Each blocker is used at most once.
    /// <para>
    /// Crude on purpose: it exists to exercise blocking and combat damage, not to play well. It reads power and
    /// toughness straight off the view, which is what the server computed, so it cannot disagree with the rules
    /// about how big anything is.
    /// </para>
    /// <para>
    /// The gang pass is what makes several creatures block one attacker, which is the only way the attacking
    /// player is ever asked for a damage assignment order (CR 509.2). Without it that prompt never arrives and
    /// the screen that answers it cannot be reached at all.
    /// </para>
    /// </summary>
    private static BlockersDeclaredAnswer Block(GameClient client, DeclareBlockersPromptMessage prompt)
    {
        var answer = new BlockersDeclaredAnswer();
        HashSet<int> used = [];
        List<ViewHandle> unblocked = [];

        foreach (ViewHandle attackerHandle in prompt.Attackers)
        {
            if (client.Object(attackerHandle) is not { } attacker)
            {
                continue;
            }

            // One creature cannot block menace or its like alone, so such an attacker goes straight to the gang pass.
            int minimum = Minimum(prompt, attackerHandle);

            ViewHandle trade = minimum > 1 ? ViewHandle.None : prompt.Eligible.FirstOrDefault(blocker =>
                !used.Contains(blocker.Value)
                && Allows(prompt, blocker, attackerHandle)
                && client.Object(blocker) is { Power: { } power, Toughness: { } toughness }
                && toughness > (attacker.Power ?? 0)
                && power >= (attacker.Toughness ?? 0));

            if (trade.IsNone)
            {
                unblocked.Add(attackerHandle);
                continue;
            }

            used.Add(trade.Value);
            answer.Blocks.Add(new BlockMessage { Blocker = trade, Attacker = attackerHandle });
        }

        foreach (ViewHandle attackerHandle in unblocked)
        {
            if (client.Object(attackerHandle) is not { } attacker)
            {
                continue;
            }

            // Two or more spare creatures whose power together is lethal. Fewer than two would be a chump block,
            // which is a decision this has no way to judge and would only lose creatures for nothing.
            List<ViewHandle> gang = [];
            int damage = 0;

            foreach (ViewHandle blocker in prompt.Eligible)
            {
                if (used.Contains(blocker.Value)
                    || !Allows(prompt, blocker, attackerHandle)
                    || client.Object(blocker)?.Power is not { } power)
                {
                    continue;
                }

                gang.Add(blocker);
                damage += power;

                if (gang.Count >= Math.Max(2, Minimum(prompt, attackerHandle)) && damage >= (attacker.Toughness ?? 0))
                {
                    break;
                }
            }

            // "Can't be blocked by more than one creature" (CR 509.1b) takes no gang.
            if (gang.Count < Math.Max(2, Minimum(prompt, attackerHandle)) || damage < (attacker.Toughness ?? 0)
                || gang.Count > prompt.MaximumFor(prompt.Attackers.IndexOf(attackerHandle)))
            {
                continue;
            }

            foreach (ViewHandle blocker in gang)
            {
                used.Add(blocker.Value);
                answer.Blocks.Add(new BlockMessage { Blocker = blocker, Attacker = attackerHandle });
            }
        }

        // Whether this obeys the restrictions and requirements (CR 509.1b-c: Lure, "blocks this turn if able") is the host's to
        // judge: GameClient.Answer settles the declaration with it, mending one it refuses from what may be added.
        return answer;
    }

    /// <summary>
    /// A triggered ability's target: an opponent or something an opponent controls if there is one. "Up to one" with
    /// nothing of an opponent's on offer chooses none, so Loran does not destroy this seat's own artifact; a required target
    /// takes what there is.
    /// </summary>
    private static List<TargetMessage> ChooseTarget(GameClient client, ChooseTargetPromptMessage prompt)
    {
        // "With total mana value X or less" (CR 601.2c): the first that fit together.
        if (prompt.MaximumTotalManaValue is { } most)
        {
            return Cantrip.Core.Decisions.TotalManaValue.FirstFitting(prompt.Candidates, prompt.ManaValues, prompt.Count, most);
        }

        // A slot that takes several — "Untap X target lands" — wants exactly that many, and whether aiming at an
        // opponent's is good or bad depends on the card, so it simply fills the slot from what is offered.
        if (prompt.Count > 1)
        {
            return [.. prompt.Candidates.Take(prompt.Count)];
        }

        TargetMessage? theirs = prompt.Candidates.FirstOrDefault(candidate => Theirs(client, candidate));

        if (theirs is not null)
        {
            return [theirs];
        }

        return prompt.Optional || prompt.Candidates.Count == 0 ? [] : [prompt.Candidates[0]];
    }

    /// <summary>Whether a candidate is an opponent or something an opponent controls.</summary>
    private static bool Theirs(GameClient client, TargetMessage candidate) =>
        candidate.IsPlayer
            ? candidate.Player != client.ActingSeat
            : client.Object(candidate.Object) is { } target && target.Controller != client.ActingSeat;

    /// <summary>The fewest blockers the prompt says this attacker needs: 2 for menace, otherwise usually 1.</summary>
    private static int Minimum(DeclareBlockersPromptMessage prompt, ViewHandle attacker)
    {
        int index = prompt.Attackers.IndexOf(attacker);
        return index >= 0 && index < prompt.MinimumBlockers.Count ? prompt.MinimumBlockers[index] : 1;
    }

    /// <summary>Whether the prompt allows this pair: flying (CR 702.9b) rules some blocks out.</summary>
    private static bool Allows(DeclareBlockersPromptMessage prompt, ViewHandle blocker, ViewHandle attacker) =>
        prompt.Allowed.Any(pair => pair.Blocker == blocker && pair.Attacker == attacker);

    /// <summary>
    /// Plays a land, taps everything, casts whatever it can afford — and only in its own precombat main phase.
    /// <para>
    /// The phase restriction is not strategy, it is the difference between working and not: mana made in an
    /// earlier step empties when that step ends (CR 500.4) and the lands stay tapped until the next untap step, so
    /// tapping outside a main phase spends the turn's resources on nothing.
    /// </para>
    /// </summary>
    private static void ActOnPriority(GameClient client, PriorityPromptMessage priority)
    {
        PlayerView? view = client.View;

        bool myMainPhase = view is not null
            && view.Step == TurnStepView.PrecombatMain
            && view.ActivePlayer == client.ActingSeat;

        if (!myMainPhase)
        {
            client.Pass();
            return;
        }

        // Lands, then mana, then spells, then whatever other abilities the mana left over will pay for: an ability such as
        // Grim Monolith's untap would otherwise spend the mana the turn's spells wanted.
        foreach ((ActionKindView kind, bool mana) in ((ActionKindView, bool)[])
            [
                (ActionKindView.PlayLand, false), (ActionKindView.ActivateAbility, true),
                (ActionKindView.CastSpell, false), (ActionKindView.ActivateAbility, false),
            ])
        {
            foreach (ActionOptionMessage option in priority.Options)
            {
                if (option.Kind != kind)
                {
                    continue;
                }

                bool addsMana = option.Produces.Count > 0 || ChoosesColors(option);
                if (kind == ActionKindView.ActivateAbility && addsMana != mana)
                {
                    continue;
                }

                // Mana that costs life or hurts is made when a spell asks for it, never ahead of time on spec.
                if (kind == ActionKindView.ActivateAbility && mana && Hurts(option))
                {
                    continue;
                }

                // Like a spell: an ability the server says cannot be paid for would be announced, given up on, and
                // announced again for ever.
                if (kind == ActionKindView.ActivateAbility && !mana && !option.Affordable)
                {
                    continue;
                }

                // A spell may be announced without the mana and taken back, but a bot that did would announce the
                // same spell again on its next turn to act and never get anywhere. The server says which it could pay
                // for; those are the only ones it tries.
                if (option.Kind == ActionKindView.CastSpell && !option.Affordable)
                {
                    continue;
                }

                // A spell with {X} that could only be cast for nothing does nothing worth a card.
                if (option.Kind == ActionKindView.CastSpell && option.HasX && option.MaxX < 1)
                {
                    continue;
                }

                // Black Lotus and its like are saved rather than tapped for mana nobody has asked for yet.
                if (option.Kind == ActionKindView.ActivateAbility && ChoosesColors(option))
                {
                    continue;
                }

                if (!TryTarget(client, option, out List<TargetMessage> targets))
                {
                    continue;
                }

                client.Answer(new TakeActionAnswer
                {
                    Kind = option.Kind,
                    Source = option.Source,
                    AbilityIndex = option.AbilityIndex,
                    OptionId = option.OptionId,
                    Tap = option.Tap,
                    Targets = targets,
                });

                return;
            }
        }

        client.Pass();
    }

    /// <summary>
    /// Pays for a spell the seat has announced: pool mana when the server asks which of it to spend, then a land
    /// that makes a colour still owed, then any land while generic is owed, and otherwise gives up.
    /// <para>
    /// Reads only what the prompt says is owed and what each option says it produces, so it never decides whether a
    /// land is the right colour — the server told it. When the pool could pay in more than one way the server offers
    /// the kinds of mana to choose between, in the order it would spend them itself, and the first is as good a
    /// choice as a bot can make. Giving up rather than tapping lands that cannot help keeps a bot from spending its
    /// turn's mana on nothing. In practice this seat taps before it casts, so the prompt is rare for it; it is here so
    /// a bot can never stall on a question it does not understand.
    /// </para>
    /// </summary>
    public static TakeActionAnswer Pay(PayCostsPromptMessage prompt, GameClient? client = null)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        if (Assisting(prompt) is { } assisting)
        {
            return assisting;
        }

        if (prompt.AnnouncesXOrKicks)
        {
            return Announce(prompt, AnnouncedValue(prompt));
        }

        if (prompt.ManaAbilities)
        {
            if (!prompt.SpendsNoMana && Owing(prompt)
                && Produce(prompt.Options, prompt.RemainingColored, prompt.RemainingGeneric, prompt.RemainingHybrid, prompt.RemainingColorless, prompt.RemainingSnow) is { } activate)
            {
                return activate;
            }

            return prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting) is { } done
                ? Answer(done) : prompt.GiveUp();
        }

        if (!prompt.HasX)
        {
            if (AssignChoice(prompt, client) is { } choice)
            {
                return Assign(choice);
            }

            // "You can't spend mana to cast this spell" (CR 601.2h, 106.4): no mana is made for it, since none of it would
            // pay. A delve exile pays {1}, a tap the {1} or a color the creature has; whatever neither covers, nothing does.
            if (prompt.SpendsNoMana)
            {
                ActionOptionMessage? notMana = ExileToPay(prompt.Options)
                    ?? prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.TapToPay)
                    ?? prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting);
                return notMana is null ? prompt.GiveUp() : Answer(notMana);
            }

            // The pool already covers it, and convoke, delve or improvise could pay some of it instead (CR 601.2h):
            // the mana is in the pool and empties at the end of the step anyway, so it is what pays.
            if (!Owing(prompt) && prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting) is { } covered)
            {
                return Answer(covered);
            }

            // CR 601.2g-h: every mana ability comes before anything is tapped or exiled to pay, after which none is
            // offered; so the mana that what could be tapped or exiled will not cover is made first.
            // Each way of paying covers no more than its own cap still allows, however many candidates it offers: a
            // {2} waterbend with five artifacts to tap pays two.
            int instead = prompt.Options
                .Where(option =>
                    (option.Kind == ActionKindView.TapToPay && option.Description.Length == 0 && MakesNoMana(prompt.Options, option))
                    || option.Kind == ActionKindView.ExileToPay)
                .GroupBy(option => option.Alternative)
                .Sum(way => Math.Min(way.Count(), way.First().PaysUpTo));
            int genericForMana = Math.Max(0, prompt.RemainingGeneric - instead);
            if ((genericForMana > 0 || prompt.RemainingColored.Count > 0 || prompt.RemainingHybrid.Count > 0 || prompt.RemainingColorless > 0 || prompt.RemainingSnow > 0)
                && (Produce(prompt.Options, prompt.RemainingColored, genericForMana, prompt.RemainingHybrid, prompt.RemainingColorless, prompt.RemainingSnow)
                    ?? (genericForMana > 0 ? Produce(prompt.Options, [], genericForMana) : null)) is { } first)
            {
                return first;
            }

            // Waterbend (CR 701.67a): a creature or artifact tapped pays {1} and makes no mana, so one that makes none
            // anyway goes first; one that could tap for mana is only tapped this way once nothing else is left.
            if (prompt.RemainingGeneric > 0 && TapToPay(prompt.Options, spareManaSources: false) is { } spare)
            {
                return Answer(spare);
            }

            // Delve (CR 702.66a): a graveyard card exiled pays {1} and spends nothing else, so it goes before mana.
            if (prompt.RemainingGeneric > 0 && ExileToPay(prompt.Options) is { } exiled)
            {
                return Answer(exiled);
            }

            if (Produce(prompt.Options, prompt.RemainingColored, prompt.RemainingGeneric, prompt.RemainingHybrid, prompt.RemainingColorless, prompt.RemainingSnow) is { } produce)
            {
                return produce;
            }

            if (prompt.RemainingGeneric > 0 && TapToPay(prompt.Options, spareManaSources: true) is { } any)
            {
                return Answer(any);
            }

            // Convoke (CR 702.51a): a creature is offered only while it could still pay some symbol, colored or {1},
            // so tapping one always pays something the mana could not.
            if (prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.TapToPay && option.Description.Length > 0) is { } convoke)
            {
                return Answer(convoke);
            }

            return prompt.GiveUp();
        }

        return PayForX(prompt);
    }

    /// <summary>
    /// The value this seat announces (CR 601.2b): as much as its mana reaches for an {X} or a kicker, and for an X paid
    /// in life — Toxic Deluge's, Fire Covenant's — half of what it could pay and live, since the server's
    /// <see cref="PayCostsPromptMessage.MostAffordable"/> is the most that leaves one life and a bot spending all of it
    /// on every such spell would hand the next attack the game.
    /// </summary>
    public static int AnnouncedValue(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        int most = Math.Max(0, prompt.MostAffordable);

        // Permanents sacrificed for a reduction each (CR 601.2b): none, a seat with no plan for its board keeping it.
        if (prompt.SacrificeReduces > 0)
        {
            return 0;
        }

        // A "−X" loyalty cost: as much as leaves the planeswalker one loyalty, so it stays on the battlefield.
        if (prompt.XRemovesLoyalty)
        {
            return Math.Max(0, most - 1);
        }

        return prompt.XPaidInLife ? most / 2 : most;
    }

    /// <summary>
    /// Announces X, or how many times to kick (CR 601.2b), as <paramref name="wanted"/>: one step towards it at a time,
    /// then announces it. Shared with <see cref="RandomPlay"/> and the model's seat, which each choose their own value.
    /// </summary>
    public static TakeActionAnswer Announce(PayCostsPromptMessage prompt, int wanted)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        if ((prompt.X < wanted || prompt.VariableKicker && wanted > 0 && !prompt.KickerPaid)
            && prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.IncreaseX) is { } raise)
        {
            return Answer(raise);
        }

        if (prompt.X > wanted && prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.DecreaseX) is { } lower)
        {
            return Answer(lower);
        }

        return prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting) is { } confirm
            ? Answer(confirm)
            : prompt.GiveUp();
    }

    /// <summary>
    /// The steps of a payment with assist (CR 702.132a) that this seat answers the plainest legal way, or null for any other
    /// payment, and for the caster's own payment, which is paid as any other is. A window for mana abilities is closed with
    /// nothing activated; the player chosen to help announces that they pay none of the generic mana, and if asked to pay
    /// some anyway, pays with the first pool mana offered. None of the chosen player's steps offers giving up: the spell
    /// is not theirs (<see cref="PayCostsPromptMessage.GiveUp"/>).
    /// </summary>
    public static TakeActionAnswer? Assisting(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        return prompt.AssistStep switch
        {
            AssistSteps.HelperActivates or AssistSteps.CasterActivates =>
                prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting) is { } done ? Answer(done) : prompt.GiveUp(),
            AssistSteps.HelperAnnounces => Announce(prompt, 0),
            AssistSteps.HelperPays => prompt.GiveUp(),
            _ => null,
        };
    }

    /// <summary>
    /// A permanent offered to pay {1} of a waterbend cost by being tapped, or null. Without
    /// <paramref name="spareManaSources"/>, only one that is not also offered as a source of mana.
    /// </summary>
    private static ActionOptionMessage? TapToPay(IReadOnlyList<ActionOptionMessage> options, bool spareManaSources) =>
        options.FirstOrDefault(option =>
            option.Kind == ActionKindView.TapToPay && (spareManaSources || MakesNoMana(options, option)));

    /// <summary>Whether no mana ability on offer is the same permanent's as this tap-to-pay option.</summary>
    private static bool MakesNoMana(IReadOnlyList<ActionOptionMessage> options, ActionOptionMessage option) =>
        !options.Any(other =>
            other.Kind == ActionKindView.ActivateAbility
            && other.Source == option.Source
            && (other.Produces.Count > 0 || ChoosesColors(other)));

    /// <summary>A graveyard card offered to pay {1} of a delve cost by being exiled, or null (CR 702.66a).</summary>
    private static ActionOptionMessage? ExileToPay(IReadOnlyList<ActionOptionMessage> options) =>
        options.FirstOrDefault(option => option.Kind == ActionKindView.ExileToPay);

    /// <summary>
    /// Pays for a spell with {X}, making X as large as it can: whatever is owed comes first, then every source and
    /// every free mana in the pool goes towards X, and then it casts. When something owed cannot be covered, X is
    /// lowered until it can be, and if even the printed symbols are out of reach the spell is given up on.
    /// </summary>
    private static TakeActionAnswer PayForX(PayCostsPromptMessage prompt)
    {
        bool owing = Owing(prompt);

        if (owing)
        {
            if (Produce(prompt.Options, prompt.RemainingColored, prompt.RemainingGeneric, prompt.RemainingHybrid, prompt.RemainingColorless, prompt.RemainingSnow) is { } produce)
            {
                return produce;
            }

            ActionOptionMessage? assign = prompt.Options.FirstOrDefault(option =>
                option.Kind == ActionKindView.AssignMana && Helps(prompt, option));

            if (assign is not null)
            {
                return Assign(assign);
            }

            return prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.DecreaseX) is { } lower
                ? Answer(lower)
                : prompt.GiveUp();
        }

        // Nothing owed at all: anything more raises X.
        if (Produce(prompt.Options, [], 1) is { } more)
        {
            return more;
        }

        // Not for a kicker: a kick costs more than one mana, so a spare mana raises it to a kick the pool cannot finish,
        // the kick is lowered again, and the same spare mana would raise it again for ever. Lands tapped above kick it
        // as far as they reach; mana already floating is left alone.
        if (prompt.Kicker.Length == 0
            && prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.AssignMana) is { } spare)
        {
            return Assign(spare);
        }

        return prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.ConfirmCasting) is { } confirm
            ? Answer(confirm)
            : prompt.GiveUp();
    }

    /// <summary>
    /// Whether anything of the cost is still owed: generic, a colored symbol, a hybrid or Phyrexian one, or a
    /// colorless one (CR 107.4c). Each has its own field on the wire, and a seat that forgot one would read a cost
    /// owing only that as paid — and, paying for X, raise X it cannot finish for ever. Shared with
    /// <see cref="RandomPlay"/> and the model seat.
    /// </summary>
    /// <summary>
    /// Whether a cast or activation offered returns a tapped creature chosen once the mana is paid (CR 601.2g-h, 602.2b: a
    /// web-slinging cast, CR 702.188a, or Urban Retreat's ability), as the server marks it: the payment has to leave one
    /// tapped (<see cref="TappedCreatureMana"/>), or the action is taken back.
    /// </summary>
    public static bool ReturnsTappedAfterMana(ActionOptionMessage option)
    {
        ArgumentNullException.ThrowIfNull(option);
        return option.TappedAfterMana;
    }

    /// <summary>
    /// For a cast or activation that returns a tapped creature (<see cref="ReturnsTappedAfterMana"/>) with none of this seat's creatures tapped, the mana ability of an untapped creature of its own:
    /// one that adds mana the cost still owes, named for that mana, or else any, whose mana is then left over (CR 601.2g lets
    /// a mana ability be activated whether or not its mana is needed) but whose creature is tapped. Null when a creature is
    /// tapped already or none has a mana ability.
    /// </summary>
    public static TakeActionAnswer? TappedCreatureMana(GameClient client, PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(prompt);

        if (client.BattlefieldOf(client.ActingSeat).Any(card => card.CardTypes.Contains(CardTypeView.Creature) && card.Tapped))
        {
            return null;
        }

        List<ActionOptionMessage> creatures = [.. prompt.Options.Where(option => option.Kind == ActionKindView.ActivateAbility
            && client.Object(option.Source) is { } source && source.CardTypes.Contains(CardTypeView.Creature) && !source.Tapped && source.Controller == client.ActingSeat)];

        foreach (ActionOptionMessage option in creatures)
        {
            List<ColorView> makes = [.. option.Produces.Where(type => type != ManaTypeView.Colorless).Select(type => (ColorView)(int)type)
                .Concat(ChoosesColors(option) ? option.ColorChoices : [])];
            ColorView? owed = prompt.RemainingColored.Cast<ColorView?>().FirstOrDefault(color => makes.Contains(color!.Value));
            if (owed is not null || prompt.RemainingGeneric > 0)
            {
                return Answer(option, owed);
            }
        }

        return creatures.Count > 0 ? Answer(creatures[0]) : null;
    }

    public static bool Owing(PayCostsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        return prompt.RemainingGeneric > 0
            || prompt.RemainingColored.Count > 0
            || prompt.RemainingHybrid.Count > 0
            || prompt.RemainingColorless > 0 || prompt.RemainingSnow > 0;
    }

    /// <summary>
    /// Whether putting this pool mana towards the cost pays some of what is owed. A colour still owed needs that
    /// colour, a {C} needs colorless (CR 107.4c), and generic takes anything once no colour is owed — so a cost owing
    /// only {C} is never offered green that would be committed, paid nothing, and be offered back again. Shared with
    /// <see cref="RandomPlay"/> and the model seat.
    /// </summary>
    public static bool Helps(PayCostsPromptMessage prompt, ActionOptionMessage option)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        ArgumentNullException.ThrowIfNull(option);

        if (option.PaySnow) { return option.Snow && prompt.RemainingSnow > 0; }
        return (prompt.RemainingGeneric > 0 && prompt.RemainingColored.Count == 0 && prompt.RemainingHybrid.Count == 0)
            || option.Produces.Any(type => prompt.RemainingColored.Contains((ColorView)(int)type))
            || option.Produces.Any(type => prompt.RemainingHybrid.Exists(symbol => symbol.Accepts((ColorView)(int)type)))
            || (prompt.RemainingColorless > 0 && option.Produces.Contains(ManaTypeView.Colorless));
    }

    private static bool Hurts(ActionOptionMessage option) => option.LifeCost > 0 || option.DamageToController > 0;

    /// <summary>Puts one mana already in the pool towards what is being paid for. Shared with <see cref="RandomPlay"/>.</summary>
    /// <summary>
    /// The pool mana to put towards a cost, of the kinds the server offers in the order it would spend them itself. Mana a
    /// rule tells apart by the permanent that produced it (an option with a <c>Source</c>, CR 106.3) goes first where that
    /// permanent's text rewards spending it on this spell ("whenever you cast an artifact or creature spell using mana
    /// produced by ..."), and last otherwise, kept for a spell it would reward. Only mana that pays something owed, where any
    /// does. Null when none is offered.
    /// </summary>
    public static ActionOptionMessage? AssignChoice(PayCostsPromptMessage prompt, GameClient? client = null)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        List<ActionOptionMessage> offered = [.. prompt.Options.Where(option => option.Kind == ActionKindView.AssignMana)];
        List<ActionOptionMessage> helping = [.. offered.Where(option => Helps(prompt, option))];
        List<ActionOptionMessage> pool = helping.Count > 0 ? helping : offered;
        return pool.FirstOrDefault(option => !option.Source.IsNone && Rewarded(client, prompt, option))
            ?? pool.FirstOrDefault(option => option.Source.IsNone)
            ?? pool.FirstOrDefault();
    }

    /// <summary>
    /// Whether the text of the permanent that produced this mana rewards spending it on the spell being paid for: "whenever
    /// you cast a[n] [types] spell using mana produced by [it]", the types read off the text (none named is any spell) and
    /// matched against the spell's.
    /// </summary>
    private static bool Rewarded(GameClient? client, PayCostsPromptMessage prompt, ActionOptionMessage option)
    {
        if (client?.Object(option.Source) is not { } source || client.Object(prompt.Spell) is not { } spell)
        {
            return false;
        }

        string text = Tactics.CardReading.Text(source).ToLowerInvariant();
        int at = text.IndexOf(" spell using mana produced by", StringComparison.Ordinal);
        int start = at < 0 ? -1 : text.LastIndexOf("cast ", at, StringComparison.Ordinal);
        if (start < 0)
        {
            return false;
        }

        string[] named = [.. text[(start + "cast ".Length)..at]
            .Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Where(word => word is not ("a" or "an" or "or" or "and/or"))];
        return named.Length == 0 || spell.CardTypes.Exists(type => named.Contains(type.ToString().ToLowerInvariant()));
    }

    public static TakeActionAnswer Assign(ActionOptionMessage option) => TakeActionAnswer.Take(option);

    /// <summary>
    /// The mana ability that helps with what is still owed: a source of an owed color, then a source of any one color
    /// told to make that color, and while only generic is owed, any source at all. Null when nothing helps.
    /// <para>
    /// Sources of any one color have to be usable here, not just saved. The server counts Black Lotus when it says a
    /// spell is affordable, so a bot that announced on that advice and then refused to tap the Lotus would give up,
    /// be offered the same spell again, and announce it again forever.
    /// </para>
    /// <para>
    /// Shared with <see cref="RandomPlay"/>: which land helps with what is owed is read off what the server says each
    /// one produces, so there is nothing about it for a second seat to decide differently.
    /// </para>
    /// </summary>
    public static TakeActionAnswer? Produce(
        List<ActionOptionMessage> options,
        List<ColorView> owedColors,
        int owedGeneric,
        List<HybridSymbolView>? owedHybrid = null,
        int owedColorless = 0,
        uint owedSnow = 0)
    {
        // Painless sources first, stably, so an Ancient Tomb is only tapped when nothing else will do; and among
        // equally painless ones, the source that could cover the fewest of the colors owed.
        //
        // That second key is not a preference, it is what makes a payable cost get paid. A dual land offers one
        // option per color it makes, so a seat that simply took the first option covering something owed would tap
        // an Elegant Parlor for the {R} a Mountain could have paid, and then owe {W} with nothing but Mountains
        // left — a cost the server rightly called affordable, given up on (CR 733.1), and offered again for ever.
        // Spending the narrow sources first keeps the flexible one for the pip only it can pay.
        // A mana ability whose mana is restricted so that it can't pay this cost (CR 106.6: Sunken Citadel's two mana for a
        // spell) is no source here: its mana would only wait in the pool.
        IEnumerable<ActionOptionMessage> activations = options
            .Where(option => option.Kind == ActionKindView.ActivateAbility && !option.PaysNothing)
            .OrderBy(Hurts)
            .ThenBy(option => Flexibility(options, option, owedColors, owedHybrid));

        if (owedColors.Count > 0)
        {
            if (activations.FirstOrDefault(option => option.Produces.Any(type => owedColors.Contains((ColorView)(int)type)))
                is { } fixedSource)
            {
                return Answer(fixedSource);
            }

            foreach (ColorView owed in owedColors)
            {
                if (activations.FirstOrDefault(option => ChoosesColors(option) && option.ColorChoices.Contains(owed))
                    is { } anyColor)
                {
                    return Answer(anyColor, owed);
                }
            }

            return null;
        }

        // A hybrid or Phyrexian symbol (CR 107.4e-f) takes one mana of any of its colors, so any source of one will
        // do. Looked for before generic, because a cost is never "nothing owed" while one of these is outstanding —
        // a bot that read only the colored list would give the spell up and announce it again for ever. (A bot never
        // chooses to pay life for one instead; it just leaves that option unused, same as any other it does not need.)
        foreach (HybridSymbolView symbol in owedHybrid ?? [])
        {
            if (activations.FirstOrDefault(option => option.Produces.Any(type => symbol.Accepts((ColorView)(int)type)))
                is { } either)
            {
                return Answer(either);
            }

            foreach (ColorView color in symbol.Colors)
            {
                if (activations.FirstOrDefault(option => ChoosesColors(option) && option.ColorChoices.Contains(color))
                    is { } anyColor)
                {
                    return Answer(anyColor, color);
                }
            }

            return null;
        }

        // A colorless symbol {C} (CR 107.4c) takes colorless mana and nothing else, so only a source of colorless helps
        // — a source of any one color never does, since colorless is not a color it could be told to make. Looked for
        // before generic for the same reason hybrid is: a cost owing only {C} is not "nothing owed".
        if (owedColorless > 0)
        {
            return activations.FirstOrDefault(option => option.Produces.Contains(ManaTypeView.Colorless)) is { } colorless
                ? Answer(colorless)
                : null;
        }

        if (owedSnow > 0)
        {
            ActionOptionMessage? snow = activations.FirstOrDefault(option => option.Snow);
            return snow is null ? null : Answer(snow, ChoosesColors(snow) ? snow.ColorChoices.FirstOrDefault() : null);
        }

        if (owedGeneric <= 0)
        {
            return null;
        }

        return activations.OrderBy(Hurts).FirstOrDefault(option => !ChoosesColors(option)) is { } plain
            ? Answer(plain)
            : activations.FirstOrDefault() is { } anyColorForGeneric
                ? Answer(anyColorForGeneric, anyColorForGeneric.ColorChoices.FirstOrDefault())
                : null;
    }

    /// <summary>
    /// Whether a mana ability leaves its colors to the answer, either as one choice for all of them or as one per mana
    /// (CR 106.1b). The two are the same question to a bot looking for a source of some color.
    /// </summary>
    private static bool ChoosesColors(ActionOptionMessage option) =>
        option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0;

    /// <summary>
    /// How many of the colors still owed this option's <em>source</em> could pay: one for a Mountain against
    /// <c>{R}{W}</c>, two for a dual that makes either.
    /// <para>
    /// Counted over the source rather than the option, because tapping a permanent for one color spends every other
    /// color it could have made. A source that can only cover one owed pip is therefore no loss to spend, and one
    /// that could cover two is worth keeping until it is the only thing left that will do.
    /// </para>
    /// </summary>
    private static int Flexibility(
        List<ActionOptionMessage> options,
        ActionOptionMessage option,
        List<ColorView> owedColors,
        List<HybridSymbolView>? owedHybrid)
    {
        List<ColorView> owed =
            [.. owedColors.Concat(owedHybrid?.SelectMany(symbol => symbol.Colors) ?? []).Distinct()];

        if (owed.Count == 0)
        {
            return 0;
        }

        return owed.Count(color => options.Any(other =>
            other.Kind == ActionKindView.ActivateAbility
            && other.Source == option.Source
            && (other.Produces.Any(type => (ColorView)(int)type == color)
                || (ChoosesColors(other) && other.ColorChoices.Contains(color)))));
    }

    /// <summary>One offered option as the answer that takes it. Shared with <see cref="RandomPlay"/>.</summary>
    public static TakeActionAnswer Answer(ActionOptionMessage option, ColorView? color = null) => new()
    {
        Kind = option.Kind,
        Source = option.Source,
        AbilityIndex = option.AbilityIndex,
        OptionId = option.OptionId,
        Tap = option.Tap,
        // An ability that leaves one color to the answer needs one named whether or not this came looking for a
        // particular one: a source that adds stated mana *and* one of any color is chosen for the mana it states, and
        // an answer that then named no color at all does not fit the option it came from, so it is refused and the
        // same activation is offered again for ever. An option with ProducesOfAnyOneColor set but nothing in
        // ColorChoices has no color to name at all — it is Squandered Resources's own candidate that could only add
        // colorless (CR 106.7), answered with Mana below instead.
        Color = option.ProducesOfAnyColorEach == 0 && option.ColorChoices.Count > 0
            ? color is { } named && option.ColorChoices.Contains(named) ? named : option.ColorChoices[0]
            : null,

        Mana = option.ProducesOfAnyOneColor > 0 && option.ColorChoices.Count == 0 ? ManaTypeView.Colorless : null,

        // "In any combination of" is one choice per mana, so the answer owes that many colors (CR 106.1b). A bot takes
        // the color it came looking for as often as the ability will give it; nothing here wants a spread, and an
        // answer short of the count would be refused and the whole activation offered again for ever.
        Colors = [.. Enumerable.Repeat(
            color is { } wanted && option.ColorChoices.Contains(wanted) ? wanted : option.ColorChoices.FirstOrDefault(),
            option.ProducesOfAnyColorEach)],

        // A cost that sacrifices a permanent of the player's choosing takes any of the ones offered.
        Sacrifice = option.SacrificeCandidates.Count > 0 ? option.SacrificeCandidates[0] : null,

        // A mana ability has no targets (CR 605.1a), so the targets its option offers are the permanents its cost taps,
        // several of the player's choosing (Supportive Parents' two creatures): any that many different ones will do.
        Targets = AutoMana.IsManaAbility(option) ? [.. option.TargetCandidates.Take(option.TargetsRequired)] : [],
    };

    /// <summary>
    /// Keeps lands while it has few of them and spells otherwise, sending the rest to the last destination — the
    /// bottom of a scry, the graveyard of a surveil, the bottom of a Stock Up.
    /// <para>
    /// Crude, like everything here, but not random: a bot that always kept everything would never exercise anywhere
    /// but the first pile. Where wanting and not wanting cannot be squared with what the destinations require, it
    /// falls back to the prompt's own default, which is always legal.
    /// </para>
    /// </summary>
    public static CardsDistributedAnswer Distribute(GameClient client, DistributeCardsPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(prompt);

        int lands = client.View?.Objects.Count(o =>
            o.Zone == ZoneKindView.Battlefield && o.Controller == client.ActingSeat && o.CardTypes.Contains(CardTypeView.Land)) ?? 0;

        bool wantsLand = lands < 4;
        int last = prompt.Destinations.Count - 1;

        var answer = new CardsDistributedAnswer
        {
            Placements = [.. prompt.Destinations.Select(_ => new List<ViewHandle>())],
        };

        foreach (ViewHandle card in prompt.Cards)
        {
            bool isLand = client.Object(card)?.CardTypes.Contains(CardTypeView.Land) == true;
            answer.Placements[isLand == wantsLand ? 0 : last].Add(card);
        }

        return prompt.Problem(answer) is null ? answer : (CardsDistributedAnswer)prompt.Default();
    }

    /// <summary>
    /// Answers an offer to pay a cost mid-resolution: pays once it can, taps a land that helps while it cannot, and
    /// declines when nothing untapped would get it there.
    /// </summary>
    public static TakeActionAnswer PayUnless(UnlessPaysPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        if (prompt.Options.FirstOrDefault(option => option.Kind == ActionKindView.PayCost) is { } pay)
        {
            return Answer(pay);
        }

        return Produce(prompt.Options, prompt.RemainingColored, prompt.RemainingGeneric, prompt.RemainingHybrid, owedSnow: prompt.RemainingSnow)
            ?? Answer(prompt.Options.First(option => option.Kind == ActionKindView.DeclineCost));
    }

    /// <summary>
    /// Decides what to aim an action at, or declines to take it.
    /// <para>
    /// Only candidates the server offered are ever chosen, so this cannot aim at something illegal; which of them is
    /// worth choosing is the only decision being made. It prefers something an opponent controls and passes over an
    /// action whose only candidates are its own, which is taste rather than a rule: a seat that bolts its own
    /// creatures stops the self-played game being a useful test of combat.
    /// </para>
    /// </summary>
    /// <returns>False when the action should be skipped in favour of the next one.</returns>
    private static bool TryTarget(GameClient client, ActionOptionMessage option, out List<TargetMessage> targets)
    {
        targets = [];

        // An empty candidate list alongside a required target means the target was optional: the server found nothing
        // to offer and still offered the action, so it goes out untargeted.
        if (option.TargetsRequired == 0 || option.TargetCandidates.Count == 0)
        {
            return true;
        }

        foreach (TargetMessage candidate in option.TargetCandidates)
        {
            if (IsOpponents(client, candidate))
            {
                targets.Add(candidate);
                return true;
            }
        }

        return false;
    }

    /// <summary>Whether a candidate is an opponent or something an opponent controls, read straight off the view.</summary>
    private static bool IsOpponents(GameClient client, TargetMessage candidate) => candidate.IsPlayer
        ? candidate.Player != client.ActingSeat
        : client.Object(candidate.Object)?.Controller is { } controller && controller != client.ActingSeat;
}

/// <summary>A seat played automatically through its own client and transport.</summary>
public sealed class AutoSeat
{
    private readonly GameClient _client;
    private readonly IAutoPlayer _player;

    /// <summary>
    /// Drafts and builds a deck when the seat is in an event, before there is a game to play. A seat whose games are
    /// narrated (<see cref="Tactics.TacticalPlay.Narrator"/>) has its picks and deck told in the same place.
    /// </summary>
    private readonly DraftPlay _drafting;

    /// <summary>How long the seat looks at a Winston pile or a grid before picking (<see cref="DraftPlay.Think"/>).</summary>
    public TimeSpan DraftThink
    {
        get => _drafting.Think;
        init => _drafting = new DraftPlay { Think = value, Narrator = _drafting.Narrator };
    }

    /// <param name="transport">This seat's own mailbox.</param>
    /// <param name="player">How it decides, defaulting to the one deterministic way (<see cref="AutoPlay"/>).</param>
    public AutoSeat(IGameTransport transport, IAutoPlayer? player = null)
        : this(new GameClient(transport), player)
    {
    }

    /// <param name="client">
    /// This seat's own client, for a seat that has to say who it is first: one joining a server over a socket, where
    /// the name, the seat asked for and any ticket are the caller's to send.
    /// </param>
    /// <param name="player">How it decides, defaulting to the one deterministic way (<see cref="AutoPlay"/>).</param>
    public AutoSeat(GameClient client, IAutoPlayer? player = null)
    {
        _client = client;
        // The engine never cuts a loop off: a seat nobody drives stops going round by itself (`LoopGuard`).
        _player = new LoopGuard(player ?? new AutoPlayer());
        _drafting = new DraftPlay { Narrator = (player as Tactics.TacticalPlay)?.Narrator };
    }

    /// <summary>The client this seat plays through, for whatever else a host reads from it: the outcome, the errors.</summary>
    public GameClient Client => _client;

    public int Seat => _client.Seat;

    public bool IsOver => _client.IsOver;

    /// <summary>What this seat was last asked, for a report about a game that stopped moving. Empty when idle.</summary>
    public string Asked => _client.Prompt?.Prompt is { } prompt ? prompt.GetType().Name : string.Empty;

    /// <summary>The question this seat answered most recently, for the same report. Empty until it has answered one.</summary>
    public string LastAnswered { get; private set; } = string.Empty;

    /// <summary>Whether this seat is part way through deciding, so a renderer can say that it is.</summary>
    public bool Thinking => _player.Thinking;

    /// <summary>Applies anything received and answers if asked. Returns true if it answered.</summary>
    public bool Step()
    {
        // Nothing is taken in while a seat is mid-decision. A seat that decides somewhere else is reading this
        // client from another thread, and a position that changed underneath it would be read half from each.
        if (!_player.Thinking)
        {
            _client.Pump();
        }

        // A narrated seat's record ends with how the game ended. Every host stops stepping a seat once its game is
        // over, so this is the last call that sees it; the narration says it once however often it is called.
        if (_client.IsOver && _player is Tactics.TacticalPlay tactical)
        {
            tactical.NarrateEnd(_client);
        }

        if (_drafting.Act(_client))
        {
            return true;
        }

        string asked = Asked;
        if (!_player.Act(_client))
        {
            return false;
        }

        LastAnswered = asked;
        return true;
    }
}
