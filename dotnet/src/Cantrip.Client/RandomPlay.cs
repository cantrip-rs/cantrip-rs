using Cantrip.Protocol;

namespace Cantrip.Client;

/// <summary>
/// A seat answered without a person at it.
/// <para>
/// Two implementations, and the difference matters to what a run is for: <see cref="AutoPlayer"/> plays the same
/// way every time, which is what a screenshot and a smoke test need, and <see cref="RandomPlay"/> plays a different
/// game every time, which is what a playtest needs.
/// </para>
/// </summary>
public interface IAutoPlayer
{
    /// <summary>Answers if the client has been asked something. Returns true if it answered.</summary>
    bool Act(GameClient client);

    /// <summary>
    /// Whether a refusal of the last answer is one this player means to answer differently.
    /// <para>
    /// A refused answer leaves the question outstanding and the server asks it again, so a player that would send
    /// the same thing back is in a loop and the run has to stop and say so. One that has another answer to try is
    /// not, and a harness that stopped on its first refusal would be stopping on a legal move it could have made
    /// instead.
    /// </para>
    /// </summary>
    bool Retrying { get; }

    /// <summary>
    /// Whether this seat is part way through deciding, and its client must be left alone until it is not.
    /// <para>
    /// False for anything that decides inside <see cref="Act"/>, which is both of the seats here. It is true only
    /// for a seat that thinks somewhere else — a language model across a network — and it is what lets one do that
    /// without freezing the window: the frame loop keeps drawing, and this seat's client is not pumped, so the
    /// thinking has a position that does not move under it.
    /// </para>
    /// </summary>
    bool Thinking => false;

    /// <summary>
    /// Whether this seat accepts the draw another player offered (docs/rules-exceptions.md, RX-4), asked once as the offer
    /// is made. Every automated seat declines for now.
    /// </summary>
    bool AcceptsDraw(GameClient client) => false;
}

/// <summary>The deterministic seat, <see cref="AutoPlay"/>, as an <see cref="IAutoPlayer"/>.</summary>
public sealed class AutoPlayer : IAutoPlayer
{
    public bool Act(GameClient client) => AutoPlay.Act(client);

    /// <summary>Never: it has one answer to any question, so a refusal of it is a fault and not a retry.</summary>
    public bool Retrying => false;
}

/// <summary>
/// Plays a seat by choosing at random among the choices the server says it has.
/// <para>
/// For playing test games: a deck, a card or an interaction is worth watching under many different lines, and
/// <see cref="AutoPlay"/> only ever takes one of them — it attacks with everything, blocks only to trade, aims at
/// the first opponent's thing it sees and casts only in its own precombat main phase. None of that is wrong, and a
/// screenshot and a smoke test need exactly that sameness; it is simply one game over and over. This one plays a
/// different game every time, so the same decklist covers holding a removal spell, aiming it somewhere odd, a
/// planeswalker attacked instead of its controller, a chump block, and a spell cast in an end step.
/// </para>
/// <para>
/// It is not an opponent worth playing against and is not meant to be. There is no evaluation anywhere in it: a
/// choice is a uniform draw from what the prompt offers, which is also why it cannot quietly avoid a screen or a
/// rule that a hand-written policy would never reach.
/// </para>
/// <para>
/// Every answer is built only out of what the prompt itself says is allowed — the eligible attackers, the allowed
/// blocker pairs, the offered targets, the options list — so the server is the only authority on legality and this
/// never has to agree with it about a rule. Where a prompt cannot say everything (a creature that must block, say,
/// is not in a blocker prompt), a refusal is answered by drawing again, and by passing rather than arguing once it
/// has drawn enough times.
/// </para>
/// <para>
/// Seeded, so a game can be replayed: the same bot seed against the same shuffle is the same game. Randomness here
/// is the point of the bot rather than a way of reaching a position — a test that needs a particular board writes a
/// <c>.cantrip</c> scenario instead.
/// </para>
/// </summary>
public sealed class RandomPlay : IAutoPlayer
{
    /// <summary>
    /// How many times one question is answered at random before this seat stops arguing and passes.
    /// <para>
    /// A prompt does not always carry every rule its answer is judged by — a creature that must block, an attack
    /// restriction — so a draw from what it does carry can still be refused. Drawing again finds a legal
    /// declaration when one is there; the prompt's own default is the way out when it is not.
    /// </para>
    /// </summary>
    private const int Draws = 3;

    /// <summary>X not yet chosen for the announcement being paid for, as opposed to chosen and zero.</summary>
    private const int Unchosen = -1;

    private readonly Random _random;

    /// <summary>The question last answered, so the same one asked again is recognised as a refusal.</summary>
    private int _askedId = -1;

    /// <summary>How many times <see cref="_askedId"/> has come back.</summary>
    private int _draws;

    /// <summary>The X this seat is paying for, chosen as the spell was announced. <see cref="Unchosen"/> until then.</summary>
    private int _targetX = Unchosen;

    /// <summary>
    /// The most X this seat draws this turn, once it has given up paying for one (<see cref="XCosts"/>); null otherwise. Read
    /// from the client before each answer, so <see cref="Decide"/> stays free of it.
    /// </summary>
    private int? _xMost;

    /// <summary>The spell whose optional cost was last decided, so one decision covers the whole payment.</summary>
    private ViewHandle _unless;

    /// <summary>Whether that decision was to pay.</summary>
    private bool _paying;

    /// <param name="seed">The seed to play from, or null for a different game every run.</param>
    public RandomPlay(int? seed = null) => _random = seed is { } fixed_ ? new Random(fixed_) : new Random();

    public bool Retrying => _draws < Draws;

    public bool Act(GameClient client)
    {
        ArgumentNullException.ThrowIfNull(client);

        if (client.IsOver || !client.IsWaitingOnMe)
        {
            return false;
        }

        PromptMessage asked = client.Prompt!;

        // The identical question, id and all, is what the server sends back when it refuses an answer, so a repeat
        // is the only notice this gets that its last draw was illegal.
        if (asked.RequestId == _askedId)
        {
            _draws++;
        }
        else
        {
            _askedId = asked.RequestId;
            _draws = 0;
        }

        _xMost = XCosts.Most(client);
        AnswerPayload answer = _draws >= Draws ? asked.Prompt.Default() : Decide(asked.Prompt);

        // Costs to attack (CR 508.1h-j): a payment given up is remembered, and the declarations of the rest of that turn
        // cost nothing, so a seat that cannot pay does not declare the same attack for ever. So is a payment for an X given
        // up, and no X as large is drawn again that turn (XCosts).
        switch (asked.Prompt, answer)
        {
            case (PayCostsPromptMessage pay, TakeActionAnswer taken):
                AttackCosts.Answered(client, pay, taken);
                XCosts.Answered(client, pay, taken);
                break;
            case (DeclareAttackersPromptMessage attackers, AttackersDeclaredAnswer declared):
                declared.Attacks = AttackCosts.Within(attackers, declared.Attacks, AttackCosts.Mana(client, attackers));
                break;
        }

        client.Answer(answer);
        return true;
    }

    /// <summary>
    /// One random answer to one question, built only from what the question offers.
    /// <para>
    /// Public and free of the client: a prompt is the whole input, so what this seat would do with any board can be
    /// asked directly in a test without a game behind it.
    /// </para>
    /// </summary>
    public AnswerPayload Decide(PromptPayload prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);

        return prompt switch
        {
            PriorityPromptMessage priority => Priority(priority),
            PayCostsPromptMessage pay => Pay(pay),
            UnlessPaysPromptMessage unless => Unless(unless),
            DeclareAttackersPromptMessage attackers => Attack(attackers),
            DeclareBlockersPromptMessage blockers => Block(blockers),
            ChooseTargetPromptMessage target => Target(target),
            ChooseDividedTargetsPromptMessage divided => Divided(divided),
            RemoveCountersPromptMessage remove => RemoveCounters(remove),
            ChooseModesPromptMessage modes => Modes(modes),
            ChooseNumberPromptMessage number => new NumberChosenAnswer
            {
                Value = checked((uint)_random.NextInt64(number.Least, (long)number.Most + 1)),
            },
            ChooseCardsPromptMessage cards => Cards(cards),
            SearchLibraryPromptMessage search => Search(search),
            DiscardPromptMessage discard => new CardsChosenAnswer { Cards = Some(discard.Hand, discard.Count) },
            ChoosePlayerPromptMessage player => new PlayerChosenAnswer { Player = Any(player.Candidates) },
            ChooseStartingPlayerPromptMessage starting => new PlayerChosenAnswer { Player = Any(starting.Candidates) },

            // Always kept. A hand thrown away at random is a card down for nothing, and the games this seat plays are
            // meant to reach the rest of the rules, not to end early for want of cards.
            MulliganPromptMessage mulligan => mulligan.Default(),
            ChooseColorsPromptMessage colors => Colors(colors),
            NameCardPromptMessage name => new CardNamedAnswer { Name = name.Names.Count > 0 ? Any(name.Names) : name.Suggestion },
            ChooseReplacementPromptMessage replacement => replacement.OccurrenceOrder
                ? new CostPaymentChosenAnswer { Key = _random.Next(Math.Max(replacement.Sources.Count, 1)).ToString() }
                : new ReplacementChosenAnswer { Index = _random.Next(Math.Max(replacement.Sources.Count, 1)) },
            OrderTriggersPromptMessage order => Order(order),
            DistributeCardsPromptMessage distribute => Distribute(distribute),

            // "You may": both ways, so a search this seat may make, a life payment it may make and a copy it may
            // make are each taken about half the time. A chain of them — Chain of Smog's copy, Lim-Dûl's Vault's
            // look again — ends because stopping is drawn eventually, which is what AutoPlay had to name each of by
            // hand to avoid.
            // Not assist's "choose another player to help pay" (CR 702.132a), declined: this seat pays for its own spells.
            MayPromptMessage may => new MayChosenAnswer { Accept = may.Verb != "Assist" && Coin() },

            // Combat damage division is left to the engine's own, which is always legal (CR 510.1c-d). Which
            // blocker gets how much is a division rather than a choice among offered things, and a random split
            // that respected lethal-in-order would be arithmetic rather than a draw.
            _ => prompt.Default(),
        };
    }

    // --- Priority ---------------------------------------------------------------------------------------------------

    /// <summary>
    /// One of everything this seat may do right now, passing included.
    /// <para>
    /// Passing is one of the draws rather than what happens when nothing else is chosen (CR 117.3d): a seat that
    /// acted whenever it could would never hold a counterspell, never leave a blocker untapped and never let an
    /// end step arrive with mana up.
    /// </para>
    /// <para>
    /// Every step, not just its own main phase, which is the whole point of drawing here: an instant cast in an
    /// upkeep or an end step, an ability activated in response to something, a land played after combat. Mana is
    /// what makes that work — it is never made here on spec, only when a cost asks for it, so nothing is wasted by
    /// a step ending (CR 500.4).
    /// </para>
    /// </summary>
    private AnswerPayload Priority(PriorityPromptMessage prompt)
    {
        List<ActionOptionMessage> offered = [.. prompt.Options.Where(Takeable)];

        int draw = _random.Next(offered.Count + 1);
        if (draw == offered.Count)
        {
            return new PassAnswer();
        }

        ActionOptionMessage option = offered[draw];

        // The X or the kicks this is cast for are drawn at the announcement, where the server says how far the mana —
        // or, for an X paid in life, the life that leaves this seat alive — reaches (CR 601.2b).
        _targetX = Unchosen;

        return new TakeActionAnswer
        {
            Kind = option.Kind,
            Source = option.Source,
            AbilityIndex = option.AbilityIndex,
            OptionId = option.OptionId,
            Targets = Targets(option),
        };
    }

    /// <summary>
    /// Whether an offered action is one this seat draws from.
    /// <para>
    /// Two things are left out, and neither is taste. A mana ability is never activated here: mana made at priority
    /// empties when the step ends (CR 500.4) and the land stays tapped until the next untap step, so a seat that
    /// tapped for mana it had not been asked for would spend the turn on nothing — mana is made while paying a
    /// cost, which is where every prompt that needs it offers the same abilities. And an action the server says
    /// cannot be paid for is not taken, because announcing it, being unable to finish and giving it up (CR 733.1)
    /// leaves the seat exactly where it was, with the same action offered again.
    /// </para>
    /// </summary>
    private static bool Takeable(ActionOptionMessage option) => option.Kind switch
    {
        // A land costs nothing, so nothing is said about affording it.
        ActionKindView.PlayLand => true,

        ActionKindView.ActivateAbility => option.Affordable && !MakesMana(option),
        ActionKindView.CastSpell => option.Affordable && (!option.HasX || option.MaxX >= 0),
        ActionKindView.TurnFaceUp or ActionKindView.TakeCompanion or ActionKindView.Plot or ActionKindView.Suspend or ActionKindView.UnlockDoor => option.Affordable,

        // Nothing else belongs at priority. Taking back a mana ability is only ever offered after activating one,
        // which this seat does not do here.
        _ => false,
    };

    /// <summary>Whether an ability adds mana, either of stated types or of colors its answer would name (CR 106.1b).</summary>
    private static bool MakesMana(ActionOptionMessage option) =>
        option.Produces.Count > 0 || option.ProducesOfAnyOneColor > 0 || option.ProducesOfAnyColorEach > 0 || option.ColorChoices.Count > 0;

    /// <summary>
    /// What to aim an action at: a draw from the targets the server offered, and for "up to one target" the choice
    /// of aiming at nothing as well (CR 601.2c).
    /// </summary>
    private List<TargetMessage> Targets(ActionOptionMessage option)
    {
        if (option.TargetsRequired == 0 || option.TargetCandidates.Count == 0)
        {
            return [];
        }

        return option.TargetsOptional && _random.Next(option.TargetCandidates.Count + 1) == 0
            ? []
            : Some(option.TargetCandidates, option.TargetsRequired);
    }

    // --- Paying ------------------------------------------------------------------------------------------------------

    /// <summary>
    /// Pays for what has been announced, and for a cost with {X} chooses how large X is: at the announcement a cast or
    /// an activation asks first (CR 601.2b), or, for a special action's {X}, while it is paid as below.
    /// <para>
    /// X is raised a step at a time, which the server offers only while a larger X could still be paid for, and the
    /// spell is cast only when casting is offered — which it is only once nothing is owed. So the value is checked
    /// by the same highlighted "okay" a person waits for rather than by this counting mana, and a value that turns
    /// out to be out of reach is lowered until it is not.
    /// </para>
    /// <para>
    /// A kicker or a buyback is the same question in the same prompt (X counts the times it is paid), but it is not
    /// {X} in a mana cost, so no ceiling was offered when it was announced. There the draw is made here: raise, or
    /// stop, as a coin says, which stops on its own because the raise stops being offered.
    /// </para>
    /// <para>
    /// Covering what is owed is <see cref="AutoPlay"/>'s: which land helps is read off what the server says each
    /// one produces, and there is nothing to choose between two that both do.
    /// </para>
    /// </summary>
    private AnswerPayload Pay(PayCostsPromptMessage prompt)
    {
        // Helping pay for another player's spell, or the windows of one's own (CR 702.132a): the plainest legal answers.
        if (AutoPlay.Assisting(prompt) is { } assisting)
        {
            return assisting;
        }

        // CR 601.2b: X, or how many times to kick, is announced before the targets and the payment.
        if (prompt.AnnouncesXOrKicks)
        {
            if (_targetX is Unchosen)
            {
                // Drawn here, and drawn once, within what the server says could be paid: for an X paid in life, what
                // could be paid and survived (CR 704.5a), so a random seat never pays itself out of the game.
                int most = Math.Max(0, prompt.MostAffordable);
                if (_xMost is { } ceiling)
                {
                    most = Math.Max(prompt.LeastAnnounced, Math.Min(most, ceiling));
                }

                _targetX = prompt.VariableKicker
                    ? (Coin() ? _random.Next(most + 1) : 0)
                    : prompt.Kicker.Length > 0
                    ? Math.Min(most, prompt.Multikicker ? _random.Next(3) : Coin() ? 1 : 0)
                    : _random.Next(most + 1);
            }

            return AutoPlay.Announce(prompt, _targetX);
        }

        if (prompt.ManaAbilities)
        {
            return AutoPlay.Pay(prompt);
        }

        if (!prompt.HasX)
        {
            return AutoPlay.Pay(prompt);
        }

        bool owing = AutoPlay.Owing(prompt);
        if (_targetX is Unchosen)
        {
            // Nothing offered a ceiling when this was announced — a kicker or a buyback (CR 702.33, 702.27) rather
            // than {X} in a mana cost — so the draw is made here, and made once: whether to pay the optional cost at
            // all, and for a multikicker how many times. Drawn once rather than at every ask, because a value drawn
            // again after failing to pay for it is how a payment becomes a loop.
            _targetX = prompt.X + (prompt.Multikicker ? _random.Next(3) : Coin() ? 1 : 0);
        }

        if (prompt.X < _targetX && Offered(prompt, ActionKindView.IncreaseX) is { } raise)
        {
            return AutoPlay.Answer(raise);
        }

        if (owing)
        {
            OwedMana owed = OwedMana.Of(prompt);
            if (AutoPlay.Produce(prompt.Options, owed.Colored, owed.Generic, owed.Hybrid, owed.Colorless, owed.Snow) is { } produce)
            {
                return produce;
            }

            // Pool mana that would pay something still owed. Assigning it can raise X on its own, which is why the
            // chosen value is an aim rather than a cap.
            ActionOptionMessage? assign = prompt.Options.FirstOrDefault(option =>
                option.Kind == ActionKindView.AssignMana && AutoPlay.Helps(prompt, option));

            if (assign is not null)
            {
                return AutoPlay.Assign(assign);
            }

            // Out of reach at this X, so down one — and the value aimed at comes down with it. Leaving the aim where
            // it was would raise X again on the next ask, fail to pay for it again, and lower it again for ever: the
            // value has to be given up on, not just the attempt at it. With X already nothing there is nothing to
            // lower, so the spell itself is given up on (CR 733.1).
            if (Answer(prompt, ActionKindView.DecreaseX) is { } lower)
            {
                _targetX = prompt.X - 1;
                return lower;
            }

            return prompt.GiveUp();
        }

        return Answer(prompt, ActionKindView.ConfirmCasting) ?? prompt.GiveUp();
    }

    /// <summary>
    /// Whether to pay an optional cost that stops something happening (CR 118.12), such as Mana Leak's {3}.
    /// <para>
    /// Drawn once per spell asking, not once per prompt: paying takes several answers, and a coin flipped at each
    /// of them would tap lands for a cost it then declined.
    /// </para>
    /// </summary>
    private AnswerPayload Unless(UnlessPaysPromptMessage prompt)
    {
        if (prompt.Source != _unless)
        {
            _unless = prompt.Source;
            _paying = Coin();
        }

        // "You may pay {X}" with X its own to choose (CR 107.3f), or a cost paid up to a number of times (CR 603.12a): now and
        // then raised by one before paying, so the number is drawn too.
        if (_paying && (prompt.ChoosingX || prompt.ChoosingTimes) && Coin() && prompt.Options.Find(option => option.Kind == ActionKindView.IncreaseX) is { } raise)
        {
            return AutoPlay.Answer(raise);
        }

        return _paying
            ? AutoPlay.PayUnless(prompt)
            : AutoPlay.Answer(prompt.Options.First(option => option.Kind == ActionKindView.DeclineCost));
    }

    // --- Combat ------------------------------------------------------------------------------------------------------

    /// <summary>
    /// Declares attackers by giving every creature that could attack its own draw: stay home, or attack any one of
    /// the seats it may, or any one planeswalker an opponent controls (CR 508.1, 508.1b).
    /// <para>
    /// A creature that must attack (CR 508.1d) does not get the choice of staying home, and one whose requirement
    /// names which seats it may attack — a goaded creature (CR 701.15b) — draws from those, which rules out the
    /// planeswalkers that would leave the requirement unmet.
    /// </para>
    /// <para>
    /// So an attack is a fresh decision per creature rather than all-or-nothing: some turns everything comes, some
    /// turns one creature comes and the rest stay back, and some turns a planeswalker is the one attacked. None of
    /// those is reachable from a seat that always attacks with everything.
    /// </para>
    /// </summary>
    private AnswerPayload Attack(DeclareAttackersPromptMessage prompt)
    {
        var answer = new AttackersDeclaredAnswer();

        foreach (ViewHandle attacker in prompt.Eligible)
        {
            // What the host's verdict on no attacks offers it (CR 508.1c-d), and staying home where the engine's own declaration
            // does not have it attack - a preference only: whether the declaration is legal is the host's verdict, and
            // GameClient.Answer mends one it refuses.
            List<AttackMessage?> choices = [];

            if (!prompt.IsRequired(attacker))
            {
                choices.Add(null);
            }

            List<AttackMessage> offered = prompt.OfferedFor(attacker);
            choices.AddRange(offered.Count > 0 ? offered! : prompt.AttacksFor(attacker)!);

            if (choices.Count > 0 && Any(choices) is { } attack)
            {
                answer.Attacks.Add(attack);
            }
        }

        // What it can pay for (CR 508.1h): an attack whose cost is given up is declared again, and may come out the same.
        answer.Attacks = AttackCosts.Within(prompt, answer.Attacks, prompt.CanPay < 0 ? int.MaxValue : prompt.CanPay);
        return answer;
    }

    /// <summary>
    /// Declares blockers by giving every creature that could block its own draw: stand by, or block any one of the
    /// attackers the prompt allows it to (CR 509.1a-b) - or, for a creature that can block more than one
    /// (<see cref="DeclareBlockersPromptMessage.CapacityOf"/>), a coin for each attacker, up to as many as it can block.
    /// <para>
    /// Several blockers landing on one attacker is how a gang block happens, and with it the attacker's division of
    /// damage among them (CR 510.1c) — which a seat that only ever blocked to trade could not reach.
    /// </para>
    /// <para>
    /// Blocks short of a minimum are taken back rather than sent: menace and "except by three or more" are stated
    /// per attacker, and a declaration giving one fewer than it needs is refused as a whole (CR 509.1b), which
    /// would throw away the blocks on every other attacker with it.
    /// </para>
    /// </summary>
    private AnswerPayload Block(DeclareBlockersPromptMessage prompt)
    {
        var answer = new BlockersDeclaredAnswer();

        foreach (ViewHandle blocker in prompt.Eligible)
        {
            List<ViewHandle> attackers =
                [.. prompt.Allowed.Where(pair => pair.Blocker == blocker).Select(pair => pair.Attacker)];

            // A creature that can block more than one attacker (CR 509.1a) blocks each it may by a coin, as many as it can.
            int capacity = prompt.CapacityOf(blocker);
            if (capacity > 1)
            {
                foreach (ViewHandle attacker in attackers.Where(_ => _random.Next(2) == 0).Take(capacity))
                {
                    answer.Blocks.Add(new BlockMessage { Blocker = blocker, Attacker = attacker });
                }

                continue;
            }

            int draw = _random.Next(attackers.Count + 1);
            if (draw < attackers.Count)
            {
                answer.Blocks.Add(new BlockMessage { Blocker = blocker, Attacker = attackers[draw] });
            }
        }

        for (int i = 0; i < prompt.Attackers.Count; i++)
        {
            ViewHandle attacker = prompt.Attackers[i];
            int minimum = i < prompt.MinimumBlockers.Count ? prompt.MinimumBlockers[i] : 1;
            int blocking = answer.Blocks.Count(block => block.Attacker == attacker);

            if (blocking > 0 && blocking < minimum)
            {
                answer.Blocks.RemoveAll(block => block.Attacker == attacker);
            }

            // More than it may be blocked by ("can't be blocked by more than one creature", CR 509.1b): the first stay.
            int most = prompt.MaximumFor(i);
            if (blocking > most)
            {
                List<BlockMessage> over = [.. answer.Blocks.Where(block => block.Attacker == attacker).Skip(most)];
                answer.Blocks.RemoveAll(over.Contains);
            }
        }

        // Blocks an effect requires (CR 509.1c: Lure, "blocks this turn if able") are the host's to judge: GameClient.Answer
        // settles the declaration with it, mending one it refuses from what may be added.
        return answer;
    }

    // --- Choosing ----------------------------------------------------------------------------------------------------

    /// <summary>
    /// A draw from the targets the server offered, with aiming at nothing among them where that is allowed.
    /// <para>
    /// No preference for an opponent's things, which is the point: a seat that only ever aimed at an opponent never
    /// exercises a card aimed at its own, and half of what a removal spell can be pointed at is its controller's.
    /// </para>
    /// </summary>
    private AnswerPayload Target(ChooseTargetPromptMessage prompt)
    {
        if (prompt.Candidates.Count == 0)
        {
            return new TargetsChosenAnswer();
        }

        if (prompt.Optional && _random.Next(prompt.Candidates.Count + 1) == 0)
        {
            return new TargetsChosenAnswer();
        }

        // A printed range, "one or two target creatures" (CR 601.2c), takes any number from its minimum to its count.
        int fewest = prompt.Optional ? 0 : prompt.Minimum > 0 ? prompt.Minimum : prompt.Count;
        int number = Math.Min(prompt.Candidates.Count, fewest + _random.Next(prompt.Count - fewest + 1));

        List<TargetMessage> drawn = Some(prompt.Candidates, Math.Max(number, 1));

        // "With total mana value X or less" (CR 601.2c): the draw cut back to those that fit together, in the order drawn.
        if (prompt.MaximumTotalManaValue is { } most)
        {
            List<int> values = [.. drawn.Select(target => prompt.ManaValues.ElementAtOrDefault(prompt.Candidates.IndexOf(target)))];
            drawn = Cantrip.Core.Decisions.TotalManaValue.FirstFitting(drawn, values, drawn.Count, most);
        }

        return new TargetsChosenAnswer { Targets = drawn };
    }

    /// <summary>
    /// Divides a total among a drawn number of the offered targets, each getting at least one of it (CR 601.2d), so
    /// no more of them are chosen than the total can cover however many the slot would allow.
    /// </summary>
    private AnswerPayload Divided(ChooseDividedTargetsPromptMessage prompt)
    {
        if (prompt.Total <= 0 || prompt.Candidates.Count == 0)
        {
            return prompt.Default();
        }

        int most = Math.Min(
            prompt.Total,
            prompt.MaxTargets > 0 ? Math.Min(prompt.MaxTargets, prompt.Candidates.Count) : prompt.Candidates.Count);

        List<TargetShareMessage> shares =
        [
            .. Some(prompt.Candidates, 1 + _random.Next(most)).Select(target =>
                new TargetShareMessage { Target = target, Amount = 1 }),
        ];

        for (int left = prompt.Total - shares.Count; left > 0; left--)
        {
            shares[_random.Next(shares.Count)].Amount++;
        }

        return new TargetsDividedAnswer { Shares = shares };
    }

    /// <summary>Counters removed from among permanents (CR 118.3): each one drawn from the pools with some left.</summary>
    private AnswerPayload RemoveCounters(RemoveCountersPromptMessage prompt)
    {
        int[] taken = new int[prompt.Pools.Count];
        for (int i = 0; i < prompt.Total; i++)
        {
            List<int> open = [.. Enumerable.Range(0, prompt.Pools.Count).Where(p => taken[p] < prompt.Pools[p].Count)];
            if (open.Count == 0)
            {
                return prompt.Default();
            }

            taken[open[_random.Next(open.Count)]]++;
        }

        return new CountersRemovedAnswer
        {
            Removals = [.. Enumerable.Range(0, prompt.Pools.Count).Where(p => taken[p] > 0).Select(p =>
                new CounterPoolMessage { Object = prompt.Pools[p].Object, Counter = prompt.Pools[p].Counter, Count = taken[p] })],
        };
    }

    /// <summary>Drawn modes, as many as the choice asks for, or a drawn number of them where it offers a range.</summary>
    private AnswerPayload Modes(ChooseModesPromptMessage prompt)
    {
        if (prompt.Eligible.Count == 0)
        {
            return prompt.Default();
        }

        int count = prompt.Most > prompt.Count
            ? prompt.Count + _random.Next(prompt.Most - prompt.Count + 1)
            : prompt.Count;

        return prompt.MayRepeat
            ? new ModesChosenAnswer { Modes = [.. Enumerable.Range(0, count).Select(_ => Any(prompt.Eligible))] }
            : new ModesChosenAnswer { Modes = Some(prompt.Eligible, count) };
    }

    /// <summary>
    /// Drawn cards: as many as the choice asks for, or up to that many where fewer are allowed.
    /// <para>
    /// A choice that has to add up to a total power — saddle's (CR 702.171a) — is drawn as an order rather than as a
    /// subset: the candidates are shuffled and taken until the total is met, so which creatures saddle differs run
    /// to run while the answer still meets what was asked.
    /// </para>
    /// </summary>
    private AnswerPayload Cards(ChooseCardsPromptMessage prompt)
    {
        if (prompt.MinimumPower > 0)
        {
            List<int> order = Shuffled(prompt.Cards.Count);
            List<ViewHandle> taken = [];
            int power = 0;

            foreach (int index in order)
            {
                if (power >= prompt.MinimumPower)
                {
                    break;
                }

                int each = index < prompt.Powers.Count ? prompt.Powers[index] : 0;
                if (each <= 0)
                {
                    // A creature with no power adds nothing, and a negative one takes from the total (CR 107.1b).
                    continue;
                }

                taken.Add(prompt.Cards[index]);
                power += each;
            }

            return power >= prompt.MinimumPower
                ? new CardsChosenAnswer { Cards = taken }
                : prompt.Default();
        }

        if (prompt.MinimumManaValue > 0)
        {
            // Collect evidence's total mana value (CR 701.59a), drawn the same way.
            List<ViewHandle> taken = [];
            int total = 0;

            foreach (int index in Shuffled(prompt.Cards.Count))
            {
                if (total >= prompt.MinimumManaValue)
                {
                    break;
                }

                taken.Add(prompt.Cards[index]);
                total += index < prompt.ManaValues.Count ? prompt.ManaValues[index] : 0;
            }

            return total >= prompt.MinimumManaValue
                ? new CardsChosenAnswer { Cards = taken }
                : prompt.Default();
        }

        if (prompt.MaximumTotalManaValue is { } limit)
        {
            // "With total mana value N or less" (CR 202.3): a shuffled order, each card taken while it still fits, up to a drawn
            // number of them. The rule's verdicts judge the cards (GameClient.Answer).
            List<ViewHandle> eligible = prompt.Verdict.Addable;
            List<int> order = Shuffled(eligible.Count);
            int wanted = _random.Next(Math.Min(prompt.Count, eligible.Count) + 1);
            List<ViewHandle> taken = [];
            int left = limit;

            foreach (int index in order)
            {
                int at = prompt.Cards.IndexOf(eligible[index]);
                int value = at >= 0 && at < prompt.ManaValues.Count ? prompt.ManaValues[at] : 0;
                if (taken.Count < wanted && value <= left)
                {
                    taken.Add(eligible[index]);
                    left -= value;
                }
            }

            return new CardsChosenAnswer { Cards = taken };
        }

        // "Up to" a number takes any number down to the least the prompt allows: none, or craft's "one or more" (CR 702.167a).
        int most = Math.Min(prompt.Count, prompt.Verdict.Addable.Count);
        int least = Math.Min(prompt.Minimum, most);
        List<ViewHandle> drawn = Some(prompt.Verdict.Addable, prompt.UpTo ? least + _random.Next(most - least + 1) : most);

        // A band (CR 702.22c) has at most one creature without banding: the first drawn stays.
        if (prompt.Purpose == CardChoicePurposeView.Band)
        {
            List<ViewHandle> without = [.. drawn.Where(prompt.WithoutBanding.Contains).Skip(1)];
            drawn.RemoveAll(without.Contains);
        }

        return new CardsChosenAnswer { Cards = drawn };
    }

    /// <summary>Drawn cards from a library search, including finding nothing where that is allowed (CR 701.19d).</summary>
    private AnswerPayload Search(SearchLibraryPromptMessage prompt)
    {
        int most = Math.Min(prompt.Count, prompt.Verdict.Addable.Count);
        return new CardsChosenAnswer { Cards = Some(prompt.Verdict.Addable, prompt.Optional ? _random.Next(most + 1) : most) };
    }

    /// <summary>Drawn colors, repeats and all where the choice allows them.</summary>
    private AnswerPayload Colors(ChooseColorsPromptMessage prompt)
    {
        if (prompt.Choices.Count == 0)
        {
            return prompt.Default();
        }

        return prompt.MayRepeat
            ? new ColorsChosenAnswer { Colors = [.. Enumerable.Range(0, prompt.Count).Select(_ => Any(prompt.Choices))] }
            : new ColorsChosenAnswer { Colors = Some(prompt.Choices, prompt.Count) };
    }

    /// <summary>
    /// A drawn order for abilities that triggered at once (CR 603.3b). Any order is legal, and the order decides
    /// which resolves first, so this is a real difference between two runs and not a formality.
    /// </summary>
    private AnswerPayload Order(OrderTriggersPromptMessage prompt)
    {
        var order = new TriggersOrderedAnswer();
        order.Order.AddRange(Shuffled(prompt.Sources.Count));
        return order;
    }

    /// <summary>
    /// A drawn placement of cards among a scry's, a surveil's or a Stock Up's destinations, checked against the
    /// prompt's own rule and given up on when it does not hold — some of them require an exact number in a pile.
    /// </summary>
    private AnswerPayload Distribute(DistributeCardsPromptMessage prompt)
    {
        var answer = new CardsDistributedAnswer
        {
            Placements = [.. prompt.Destinations.Select(_ => new List<ViewHandle>())],
        };

        if (answer.Placements.Count == 0)
        {
            return prompt.Default();
        }

        foreach (ViewHandle card in prompt.Cards)
        {
            answer.Placements[_random.Next(answer.Placements.Count)].Add(card);
        }

        return prompt.Problem(answer) is null ? answer : prompt.Default();
    }

    // --- Drawing -----------------------------------------------------------------------------------------------------

    private bool Coin() => _random.Next(2) == 0;

    /// <summary>One of them, uniformly.</summary>
    private T Any<T>(IReadOnlyList<T> choices) => choices[_random.Next(choices.Count)];

    /// <summary>
    /// A drawn <paramref name="count"/> of them, each at most once and never more than there are, in a drawn order —
    /// which matters where the answer is read in order, as a distribution is.
    /// </summary>
    private List<T> Some<T>(IReadOnlyList<T> choices, int count) =>
        [.. Shuffled(choices.Count).Take(Math.Clamp(count, 0, choices.Count)).Select(index => choices[index])];

    /// <summary>The indexes 0 to <paramref name="count"/>, shuffled.</summary>
    private List<int> Shuffled(int count)
    {
        List<int> order = [.. Enumerable.Range(0, Math.Max(count, 0))];

        for (int i = order.Count - 1; i > 0; i--)
        {
            int j = _random.Next(i + 1);
            (order[i], order[j]) = (order[j], order[i]);
        }

        return order;
    }

    private static ActionOptionMessage? Offered(PayCostsPromptMessage prompt, ActionKindView kind) =>
        prompt.Options.FirstOrDefault(option => option.Kind == kind);

    private static TakeActionAnswer? Answer(PayCostsPromptMessage prompt, ActionKindView kind) =>
        Offered(prompt, kind) is { } option ? AutoPlay.Answer(option) : null;
}
