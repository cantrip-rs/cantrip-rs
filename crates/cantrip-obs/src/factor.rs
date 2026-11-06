//! The answer factoring (docs/ai-player-plan.md, "Answers"): a request's answer made as a few small choices. For an
//! `Ask`, a `Machine` offers one `Step` at a time: option tokens, each a kind and the canonical objects, players or
//! numbers it names, and a mask of those the request's local constraints allow. Choosing a token moves the machine on;
//! when it has nothing left to ask it holds a `FactoredAnswer`, the answer in canonical terms. Every machine finishes
//! within `Machine::bound` steps (asserted).
//!
//! What a mask cannot see, because it is a property of the whole answer and not of the next choice, is checked when the
//! answer is validated, and an answer may be refused for it (the caller masks that answer and backtracks):
//! - blockers: a creature with menace (or "can't be blocked except by N") blocked by too few, and the block
//!   requirements' maximum (CR 509.1c);
//! - attackers: where a cap on how many may attack a defender (CR 508.1c) leaves a declaration short of the most
//!   requirements it must obey (CR 508.1d); without a cap the mask carries the requirements exactly. A creature that
//!   can't attack alone (CR 508.1c) is masked exactly: a lone attacker is left only where another can still join it;
//! - a band with more than one creature without banding (CR 702.22c);
//! - targets that break their slot's rule together past a running total of mana value (different mana values: Agadeem's
//!   Awakening), and cards that break a selection rule (CR 601.2c);
//! - a chosen name that is no card name or not of the kind asked for (CR 201.4);
//! - a total power or mana value short of its minimum (tapping for power, collecting evidence), when everything was
//!   chosen and it is still short;
//! - where the choices run out before the request is met (more wanted than there is to choose), the answer as it is.
//!
//! `decompose` is the inverse: the tokens that make a given answer, by one canonical rule. A list the answer orders
//! (cards, targets, modes, colors, trigger order, placements) is chosen in the answer's own order, placements
//! destination by destination; a declaration per object (attackers, blockers, damage shares, divided amounts, counters
//! removed) is walked in canonical object order, a unit at a time where it is an amount; X is the highest bucket at or
//! under the X wanted; a payment is `AutoPay`.

use serde::{Deserialize, Serialize};

use crate::{Ask, AskKind, OptionObs, Ref, action};

/// The values of X (or a kicker count) a policy chooses among: every value to ten, then wider steps. Those over what
/// the seat could pay (`x_most`), or that the request cannot move to, are masked.
pub const X_BUCKETS: [u32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 30];

/// A color token's number: 0 to 4 are white, blue, black, red, green; `COLORLESS` names colorless (CR 106.7); `NO_COLOR`
/// names nothing.
pub const COLORLESS: u32 = 5;
pub const NO_COLOR: u32 = 6;

/// What an option token does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Tok {
    /// Give up the spell or ability being announced. Offered on the first step where the request may be cancelled.
    Cancel,
    /// Stop choosing: the cards, targets or modes chosen so far are the answer.
    Done,
    /// Priority: pass.
    Pass,
    /// Priority: take option `n` (`a` its source).
    Option,
    /// The option's next target, `a`.
    Target,
    /// A color, `n` (see `COLORLESS`, `NO_COLOR`).
    Color,
    /// Pay the cost the pool and mana abilities can pay; payment is no policy step.
    AutoPay,
    /// X (or the kicker count) is `n`.
    X,
    /// Pay an "unless pays" cost.
    Pay,
    /// Decline it.
    Decline,
    /// Choose card or target `a`.
    Pick,
    /// Choose mode `n`.
    Mode,
    Yes,
    No,
    /// Player `a` (`n` its seat).
    Player,
    /// The name `n` of the caller's list.
    Name,
    /// Name nothing.
    NoName,
    /// Subtype `n`.
    Subtype,
    /// Creature `a` does not attack.
    Stay,
    /// Creature `a` attacks `b`.
    Attack,
    /// Creature `a` does not block.
    NoBlock,
    /// Creature `a` blocks attacker `b` (`n` its index among the attackers).
    Block,
    /// One of `a`'s combat damage to creature `b` (`n` its index in the division), or with `b` none to the defender.
    Damage,
    /// One unit to `a` (`n` its index): a divided amount to a target, a counter from a pool.
    Unit,
    /// Card `a` to destination `n`, under (or after) those put there before it.
    Place,
    /// Shuffle instead of placing.
    Shuffle,
    /// Trigger `n` goes on the stack next.
    Next,
    /// Replacement effect `n`.
    Index,
    /// A base-16 digit of a cost-payment operation index, most significant first.
    CostDigit,
}

/// An option token: a kind, the canonical objects or players it names, and a small number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Token {
    pub kind: Tok,
    pub a: Ref,
    pub b: Ref,
    pub n: u32,
}

impl Token {
    pub const fn new(kind: Tok) -> Token {
        Token {
            kind,
            a: Ref::None,
            b: Ref::None,
            n: 0,
        }
    }

    pub const fn of(kind: Tok, a: Ref, b: Ref, n: u32) -> Token {
        Token { kind, a, b, n }
    }

    fn on(kind: Tok, a: Ref) -> Token {
        Token {
            kind,
            a,
            b: Ref::None,
            n: 0,
        }
    }

    fn n(kind: Tok, n: u32) -> Token {
        Token {
            kind,
            a: Ref::None,
            b: Ref::None,
            n,
        }
    }
}

/// One step: the tokens offered, and which of them are legal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub tokens: Vec<Token>,
    pub mask: Vec<bool>,
}

impl Step {
    fn push(&mut self, token: Token, legal: bool) {
        self.tokens.push(token);
        self.mask.push(legal);
    }

    /// The legal tokens' indices.
    pub fn legal(&self) -> impl Iterator<Item = usize> + '_ {
        self.mask.iter().enumerate().filter(|m| *m.1).map(|m| m.0)
    }

    /// With nothing else legal, `Done` is: a request that wants more than there is to choose is answered as it is.
    fn or_done(mut self) -> Step {
        if !self.mask.iter().any(|&m| m)
            && let Some(i) = self.tokens.iter().position(|t| t.kind == Tok::Done)
        {
            self.mask[i] = true;
        }
        self
    }
}

/// An answer in canonical terms, made by a machine. Objects are by canonical index; options, triggers and replacement
/// effects by their place in the ask's (sorted) lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactoredAnswer {
    Cancel,
    Pass,
    /// Priority option `option`, its targets in order, and the color named where it names one (a color token's number).
    Act {
        option: u32,
        targets: Vec<Ref>,
        color: Option<u32>,
    },
    /// Pay the cost being paid.
    AutoPay,
    /// Announce X (or the kicker count) as this, then pay.
    X(u32),
    /// An "unless pays" cost: pay it (at X, where it has one) or decline.
    Unless {
        pay: bool,
        x: Option<u32>,
    },
    /// Cards or targets, in the order chosen.
    Refs(Vec<Ref>),
    Modes(Vec<u8>),
    /// Colors by number, 0 to 4 white to green.
    Colors(Vec<u8>),
    Yes,
    No,
    Player(u8),
    Name(Option<String>),
    Subtype(u32),
    /// Attacking creatures and what each attacks.
    Attack(Vec<(Ref, Ref)>),
    /// Blocking creatures and the attacker each blocks.
    Block(Vec<(Ref, Ref)>),
    /// Combat damage: (source, the creature dealt it or `None` for the defender, amount).
    Shares(Vec<(Ref, Option<Ref>, u32)>),
    /// Each destination's cards in placing order; or shuffle.
    Placements {
        placements: Vec<Vec<Ref>>,
        shuffle: bool,
    },
    Divided(Vec<(Ref, u32)>),
    /// Counters removed: (permanent, counter kind, how many).
    Counters(Vec<(Ref, u16, u32)>),
    /// Triggers by their place in the ask, in the order put on the stack.
    Order(Vec<u8>),
    Index(u8),
    /// An exact numeric choice, never rounded to an X bucket or mode offset.
    Number(u32),
    CostPayment(usize),
}

enum Stop {
    Ask(Step),
    Cancel,
}

type Flow<T> = Result<T, Stop>;

/// Replays the tokens chosen so far through a request's program; at the first step with none chosen, stops with it.
struct Picker<'c> {
    chosen: &'c [Token],
    at: usize,
    cancel: bool,
}

impl Picker<'_> {
    /// Whether the next step offers to cancel the request.
    fn offers_cancel(&self) -> bool {
        self.at == 0 && self.cancel
    }

    fn pick(&mut self, mut step: Step) -> Flow<Token> {
        if self.offers_cancel() {
            step.tokens.insert(0, Token::new(Tok::Cancel));
            step.mask.insert(0, true);
        }
        assert!(
            step.mask.iter().any(|&m| m),
            "a step with nothing legal: {:?}",
            step.tokens
        );
        match self.chosen.get(self.at) {
            Some(&t) => {
                self.at += 1;
                if t.kind == Tok::Cancel {
                    Err(Stop::Cancel)
                } else {
                    Ok(t)
                }
            }
            None => Err(Stop::Ask(step)),
        }
    }
}

/// Whether the request may be cancelled: it says so, or it offers to cancel casting.
fn cancellable(ask: &Ask) -> bool {
    ask.may_cancel
        || match &ask.kind {
            AskKind::PayCosts { options, .. }
            | AskKind::UnlessPays { options, .. }
            | AskKind::Priority { options } => {
                options.iter().any(|o| o.kind == action::CANCEL_CASTING)
            }
            _ => false,
        }
}

fn offers(options: &[OptionObs], kind: u8) -> bool {
    options.iter().any(|o| o.kind == kind)
}

/// The most steps the machine for `ask` takes.
pub fn bound(ask: &Ask) -> usize {
    if snow_payment(&ask.kind) {
        let options = match &ask.kind { AskKind::PayCosts { options, .. } | AskKind::UnlessPays { options, .. } => options, _ => unreachable!() };
        return 2 + options.iter().map(|o| o.targets_required as usize).max().unwrap_or(0);
    }
    let n = match &ask.kind {
        AskKind::Priority { options } => {
            2 + options
                .iter()
                .map(|o| o.targets_required as usize)
                .max()
                .unwrap_or(0)
        }
        AskKind::PayCosts { .. } => 1,
        AskKind::ChooseNumber { .. } => 8,
        AskKind::ChooseCostPayment { .. } => usize::BITS as usize / 4,
        AskKind::UnlessPays { .. } => 2,
        AskKind::ChooseCards { count, .. }
        | AskKind::ChooseTarget { count, .. }
        | AskKind::SearchLibrary { count, .. } => *count as usize + 1,
        AskKind::ChooseModes { count, at_most, .. } => (*at_most).max(*count) as usize + 1,
        AskKind::ChooseColors { count, .. } | AskKind::DiscardToHandSize { count, .. } => {
            *count as usize
        }
        AskKind::TapForPower { cards, .. } | AskKind::CollectEvidence { cards, .. } => {
            cards.len() + 1
        }
        AskKind::Craft { maximum, .. } => *maximum as usize + 1,
        AskKind::RemoveCounters { total, .. } => *total as usize,
        AskKind::ChooseDividedTargets { total, .. } => *total as usize + 1,
        AskKind::DeclareAttackers { eligible, .. } => eligible.len(),
        // A creature that can block several attackers takes a step for each it blocks and one to stop.
        AskKind::DeclareBlockers { eligible, attackers, capacities, .. } => eligible
            .iter()
            .map(|c| match capacities.iter().find(|x| x.0 == *c) {
                Some(&(_, n)) => (n as usize).min(attackers.len()) + 1,
                None => 1,
            })
            .sum(),
        AskKind::AssignCombatDamage { divisions } => {
            divisions.iter().map(|d| d.damage as usize).sum()
        }
        AskKind::OrderTriggers { sources, .. } => sources.len(),
        AskKind::DistributeCards { cards, .. } => cards.len() + 1,
        AskKind::ChooseName { .. }
        | AskKind::ChooseSubtype { .. }
        | AskKind::May { .. }
        | AskKind::ChoosePlayer { .. }
        | AskKind::Mulligan { .. }
        | AskKind::ChooseStartingPlayer { .. }
        | AskKind::ChooseReplacement { .. } => 1,
    };
    n.max(1)
}

/// The X step: a bucket (and X as it is now), each legal where X can move to it and the seat could pay it.
fn x_step(p: &mut Picker, now: u32, most: u32, up: bool, down: bool) -> Flow<u32> {
    let mut values: Vec<u32> = X_BUCKETS.to_vec();
    values.push(now);
    values.sort_unstable();
    values.dedup();
    let mut step = Step::default();
    for n in values {
        let legal = n == now || (n > now && up && n <= most.max(now)) || (n < now && down);
        step.push(Token::n(Tok::X, n), legal);
    }
    Ok(p.pick(step)?.n)
}

/// The bucket `decompose` takes for X `want` from X `now`: the highest legal one at or under it.
fn x_bucket(now: u32, most: u32, up: bool, down: bool, want: u32) -> u32 {
    let mut best = now.min(want);
    for n in X_BUCKETS.into_iter().chain([now]) {
        let legal = n == now || (n > now && up && n <= most.max(now)) || (n < now && down);
        if legal && n <= want && n > best {
            best = n;
        }
    }
    if want < now && !down { now } else { best }
}

fn color_step(o: &OptionObs) -> Option<Step> {
    let colorless = o.kind == action::ACTIVATE_ABILITY && o.mana == Some(COLORLESS as u8);
    if o.color_choices == 0 && !colorless {
        return None;
    }
    let mut step = Step::default();
    for c in 0..5u32 {
        step.push(Token::n(Tok::Color, c), o.color_choices & (1 << c) != 0);
    }
    step.push(Token::n(Tok::Color, COLORLESS), colorless);
    step.push(
        Token::n(Tok::Color, NO_COLOR),
        o.color_choices == 0 || o.names_nothing,
    );
    Some(step)
}

fn act(o: &OptionObs, option: usize, p: &mut Picker) -> Flow<FactoredAnswer> {
    let mut targets: Vec<Ref> = Vec::new();
    while targets.len() < o.targets_required as usize {
        let mut step = Step::default();
        for &c in &o.target_candidates {
            step.push(Token::on(Tok::Target, c), !targets.contains(&c));
        }
        step.push(Token::new(Tok::Done), o.targets_optional);
        let t = p.pick(step.or_done())?;
        if t.kind == Tok::Done {
            break;
        }
        targets.push(t.a);
    }
    let color = match color_step(o) {
        Some(step) => Some(p.pick(step)?.n),
        None => None,
    };
    Ok(FactoredAnswer::Act {
        option: option as u32,
        targets,
        color,
    })
}

/// Cards or targets one at a time from `from`, each legal while unchosen and within the total, `Done` legal once
/// `least` are chosen; at most `most`.
fn picks(
    p: &mut Picker,
    from: &[Ref],
    least: usize,
    most: usize,
    total: Option<&crate::TotalObs>,
) -> Flow<Vec<Ref>> {
    let mut chosen: Vec<Ref> = Vec::new();
    let mut sum = 0u32;
    while chosen.len() < most {
        let mut step = Step::default();
        for (i, &c) in from.iter().enumerate() {
            let fits = total.is_none_or(|t| {
                sum + u32::from(t.values.get(i).copied().unwrap_or(0)) <= u32::from(t.most)
            });
            step.push(Token::on(Tok::Pick, c), !chosen.contains(&c) && fits);
        }
        step.push(Token::new(Tok::Done), chosen.len() >= least);
        let t = p.pick(step.or_done())?;
        if t.kind == Tok::Done {
            break;
        }
        if let Some(t_) = total
            && let Some(i) = from.iter().position(|&c| c == t.a)
        {
            sum += u32::from(t_.values.get(i).copied().unwrap_or(0));
        }
        chosen.push(t.a);
    }
    Ok(chosen)
}

/// Cards one at a time, `Done` legal once their values reach `minimum` (or with nothing left to choose); `one` stops at
/// one. Where no choice reaches `minimum` and the request may be cancelled, cancelling is the only answer: the cost can't
/// be paid (CR 601.2h), and the spell or ability is taken back (CR 733.1).
fn picks_to_minimum(
    p: &mut Picker,
    cards: &[Ref],
    values: &[i32],
    minimum: u32,
    one: bool,
) -> Flow<Vec<Ref>> {
    let mut chosen: Vec<Ref> = Vec::new();
    let mut sum = 0i32;
    let value = |i: usize| values.get(i).copied().unwrap_or(0);
    let most: i32 = (0..cards.len()).map(|i| value(i).max(0)).sum();
    if !one && most < minimum as i32 && p.offers_cancel() {
        let mut step = Step::default();
        for &c in cards {
            step.push(Token::on(Tok::Pick, c), false);
        }
        step.push(Token::new(Tok::Done), false);
        p.pick(step)?;
        return Err(Stop::Cancel);
    }
    while chosen.len() < cards.len() && !(one && !chosen.is_empty()) {
        // A card is offered only while the minimum stays within reach: a negative value (a creature with negative power,
        // CR 107.1b) takes from the total, and the cards left might no longer make it up. Every legal answer's picks pass
        // this in any order, since its other cards are among those left.
        let left: i32 = (0..cards.len())
            .filter(|&i| !chosen.contains(&cards[i]))
            .map(|i| value(i).max(0))
            .sum();
        let mut step = Step::default();
        for (i, &c) in cards.iter().enumerate() {
            let reach = sum + left - value(i).max(0) + value(i);
            step.push(
                Token::on(Tok::Pick, c),
                !chosen.contains(&c) && (one || reach >= minimum as i32),
            );
        }
        step.push(Token::new(Tok::Done), one || sum >= minimum as i32);
        let t = p.pick(step.or_done())?;
        if t.kind == Tok::Done {
            break;
        }
        if let Some(i) = cards.iter().position(|&c| c == t.a) {
            sum += value(i);
        }
        chosen.push(t.a);
    }
    Ok(chosen)
}

fn names_with(ask: &Ask, names: &[String]) -> Vec<String> {
    let mut all = names.to_vec();
    if let AskKind::ChooseName {
        suggestion: Some(s),
        ..
    } = &ask.kind
        && !all.contains(s)
    {
        all.push(s.clone());
    }
    all
}

/// Payment windows whose source quality or snow demand requires an explicit allocation choice.
/// Ordinary costs retain their established automatic payment factoring.
pub fn snow_payment(kind: &AskKind) -> bool {
    match kind {
        AskKind::PayCosts { announcing: true, .. } => false,
        AskKind::PayCosts { remaining, options, .. }
        | AskKind::UnlessPays { remaining, options, .. } => {
            remaining.snow != 0 || options.iter().any(|o| o.snow || o.pay_snow)
        }
        _ => false,
    }
}

fn program(ask: &Ask, names: &[String], p: &mut Picker) -> Flow<FactoredAnswer> {
    use FactoredAnswer as F;
    let yes_no = |p: &mut Picker| -> Flow<FactoredAnswer> {
        let mut step = Step::default();
        step.push(Token::new(Tok::Yes), true);
        step.push(Token::new(Tok::No), true);
        Ok(if p.pick(step)?.kind == Tok::Yes {
            F::Yes
        } else {
            F::No
        })
    };
    match &ask.kind {
        AskKind::ChooseCostPayment { choices } => {
            let Some(span) = choices.len().checked_sub(1) else { return Err(Stop::Cancel) };
            let mut prefix = 0usize;
            for shift in (0..usize::BITS / 4).rev().map(|digit| digit * 4) {
                let mut step = Step::default();
                for digit in 0..16u16 {
                    let low = ((prefix << 4) | usize::from(digit)) << shift;
                    step.push(Token::n(Tok::CostDigit, u32::from(digit)), low <= span);
                }
                prefix = (prefix << 4) | p.pick(step)?.n as usize;
            }
            Ok(F::CostPayment(prefix))
        }
        AskKind::ChooseNumber { least, most } => {
            // Eight base-16 digits encode an offset exactly. At most sixteen options fit the existing encoder budget;
            // a prefix is legal iff at least one completion lies inside the inclusive range.
            let span = u64::from(*most) - u64::from(*least);
            let mut prefix = 0u64;
            for shift in (0..8).rev().map(|digit| digit * 4) {
                let mut step = Step::default();
                for digit in 0..16 {
                    let low = ((prefix << 4) | u64::from(digit)) << shift;
                    step.push(Token::n(Tok::X, digit), low <= span);
                }
                prefix = (prefix << 4) | u64::from(p.pick(step)?.n);
            }
            Ok(F::Number((u64::from(*least) + prefix) as u32))
        }
        AskKind::Priority { options } => {
            let mut step = Step::default();
            step.push(Token::new(Tok::Pass), true);
            for (i, o) in options.iter().enumerate() {
                let targetable =
                    o.targets_optional || o.target_candidates.len() >= o.targets_required as usize;
                step.push(
                    Token::of(Tok::Option, o.source, Ref::None, i as u32),
                    targetable,
                );
            }
            let t = p.pick(step)?;
            if t.kind == Tok::Pass {
                return Ok(F::Pass);
            }
            act(&options[t.n as usize], t.n as usize, p)
        }
        AskKind::PayCosts { options, .. } | AskKind::UnlessPays { options, .. }
            if snow_payment(&ask.kind) =>
        {
            let mut step = Step::default();
            for (i, o) in options.iter().enumerate() {
                let targetable = o.targets_optional
                    || o.target_candidates.len() >= o.targets_required as usize;
                step.push(Token::of(Tok::Option, o.source, Ref::None, i as u32), targetable);
            }
            let t = p.pick(step)?;
            act(&options[t.n as usize], t.n as usize, p)
        }
        AskKind::PayCosts {
            options,
            announcing,
            x,
            x_most,
            ..
        } => {
            if *announcing {
                return Ok(F::X(x_step(
                    p,
                    *x,
                    *x_most,
                    offers(options, action::INCREASE_X),
                    offers(options, action::DECREASE_X),
                )?));
            }
            if p.cancel {
                let mut step = Step::default();
                step.push(Token::new(Tok::AutoPay), true);
                p.pick(step)?;
            }
            Ok(F::AutoPay)
        }
        AskKind::UnlessPays {
            options, x, x_most, ..
        } => {
            let payable = options.iter().any(|o| {
                matches!(
                    o.kind,
                    action::PAY_COST | action::ASSIGN_MANA | action::ACTIVATE_ABILITY
                )
            });
            let mut step = Step::default();
            step.push(Token::new(Tok::Pay), payable);
            step.push(
                Token::new(Tok::Decline),
                offers(options, action::DECLINE_COST) || !payable,
            );
            if p.pick(step)?.kind == Tok::Decline {
                return Ok(F::Unless {
                    pay: false,
                    x: None,
                });
            }
            let x = match x {
                Some(now) => Some(x_step(
                    p,
                    *now,
                    *x_most,
                    offers(options, action::INCREASE_X),
                    offers(options, action::DECREASE_X),
                )?),
                None => None,
            };
            Ok(F::Unless { pay: true, x })
        }
        AskKind::ChooseCards {
            eligible,
            count,
            up_to,
            total,
            ..
        } => {
            let least = if *up_to { 0 } else { *count as usize };
            Ok(F::Refs(picks(
                p,
                eligible,
                least,
                *count as usize,
                total.as_ref(),
            )?))
        }
        AskKind::ChooseTarget {
            candidates,
            least,
            count,
            total,
            ..
        } => Ok(F::Refs(picks(
            p,
            candidates,
            *least as usize,
            *count as usize,
            total.as_ref(),
        )?)),
        AskKind::SearchLibrary {
            eligible,
            count,
            optional,
            ..
        } => {
            let least = if *optional {
                0
            } else {
                (*count as usize).min(eligible.len())
            };
            Ok(F::Refs(picks(p, eligible, least, *count as usize, None)?))
        }
        AskKind::DiscardToHandSize { count, hand } => Ok(F::Refs(picks(
            p,
            hand,
            *count as usize,
            *count as usize,
            None,
        )?)),
        AskKind::Craft {
            cards,
            minimum,
            maximum,
        } => Ok(F::Refs(picks(
            p,
            cards,
            *minimum as usize,
            *maximum as usize,
            None,
        )?)),
        AskKind::TapForPower {
            purpose,
            cards,
            powers,
            minimum,
        } => {
            let values: Vec<i32> = powers.iter().map(|&v| i32::from(v)).collect();
            Ok(F::Refs(picks_to_minimum(
                p,
                cards,
                &values,
                u32::from(*minimum),
                purpose == "TapToReduce",
            )?))
        }
        AskKind::CollectEvidence {
            cards,
            values,
            minimum,
        } => {
            let values: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
            Ok(F::Refs(picks_to_minimum(
                p,
                cards,
                &values,
                u32::from(*minimum),
                false,
            )?))
        }
        AskKind::ChooseModes {
            eligible,
            count,
            at_most,
            repeat,
            ..
        } => {
            let most = (*at_most).max(*count) as usize;
            let mut modes: Vec<u8> = Vec::new();
            while modes.len() < most {
                let mut step = Step::default();
                for &m in eligible {
                    step.push(
                        Token::n(Tok::Mode, u32::from(m)),
                        *repeat || !modes.contains(&m),
                    );
                }
                step.push(Token::new(Tok::Done), modes.len() >= *count as usize);
                let t = p.pick(step.or_done())?;
                if t.kind == Tok::Done {
                    break;
                }
                modes.push(t.n as u8);
            }
            Ok(F::Modes(modes))
        }
        AskKind::ChooseColors { choices, count, .. } => {
            let mut colors = Vec::new();
            for _ in 0..if *choices == 0 { 0 } else { *count } {
                let mut step = Step::default();
                for c in 0..5u32 {
                    step.push(Token::n(Tok::Color, c), choices & (1 << c) != 0);
                }
                colors.push(p.pick(step)?.n as u8);
            }
            Ok(F::Colors(colors))
        }
        AskKind::ChooseName { .. } => {
            let names = names_with(ask, names);
            let mut step = Step::default();
            for i in 0..names.len() {
                step.push(Token::n(Tok::Name, i as u32), true);
            }
            step.push(Token::new(Tok::NoName), true);
            let t = p.pick(step)?;
            Ok(F::Name(
                (t.kind == Tok::Name).then(|| names[t.n as usize].clone()),
            ))
        }
        AskKind::ChooseSubtype { choices, .. } => {
            let mut step = Step::default();
            for &c in choices {
                step.push(Token::n(Tok::Subtype, u32::from(c)), true);
            }
            Ok(F::Subtype(p.pick(step)?.n))
        }
        AskKind::May { .. } | AskKind::Mulligan { .. } => yes_no(p),
        AskKind::ChoosePlayer { candidates, .. } | AskKind::ChooseStartingPlayer { candidates } => {
            let mut step = Step::default();
            for &c in candidates {
                step.push(
                    Token::of(Tok::Player, Ref::Player(c), Ref::None, u32::from(c)),
                    true,
                );
            }
            Ok(F::Player(p.pick(step)?.n as u8))
        }
        AskKind::ChooseReplacement { sources, .. } => {
            let mut step = Step::default();
            for (i, &s) in sources.iter().enumerate() {
                step.push(Token::of(Tok::Index, s, Ref::None, i as u32), true);
            }
            Ok(F::Index(p.pick(step)?.n as u8))
        }
        AskKind::RemoveCounters {
            pools,
            total,
            up_to,
        } => {
            let mut taken = vec![0u32; pools.len()];
            let mut n = 0;
            while n < *total && pools.iter().zip(&taken).any(|(pool, &t)| t < pool.count) {
                let mut step = Step::default();
                for (i, pool) in pools.iter().enumerate() {
                    step.push(
                        Token::of(Tok::Unit, pool.object, Ref::None, i as u32),
                        taken[i] < pool.count,
                    );
                }
                // "Up to" (CR 608.2d): stopping short, with none taken too, is a choice.
                if *up_to {
                    step.push(Token::new(Tok::Done), true);
                }
                let t = p.pick(step)?;
                if t.kind == Tok::Done {
                    break;
                }
                taken[t.n as usize] += 1;
                n += 1;
            }
            Ok(F::Counters(
                pools
                    .iter()
                    .zip(taken)
                    .filter(|t| t.1 > 0)
                    .map(|(pool, t)| (pool.object, pool.kind, t))
                    .collect(),
            ))
        }
        AskKind::ChooseDividedTargets {
            candidates,
            total,
            max_targets,
            optional,
        } => {
            let mut shares = vec![0u32; candidates.len()];
            let mut n = 0;
            while n < *total && !candidates.is_empty() {
                let named = shares.iter().filter(|&&s| s > 0).count();
                let mut step = Step::default();
                for (i, &c) in candidates.iter().enumerate() {
                    step.push(
                        Token::of(Tok::Unit, c, Ref::None, i as u32),
                        shares[i] > 0 || *max_targets == 0 || named < *max_targets as usize,
                    );
                }
                step.push(Token::new(Tok::Done), n == 0 && *optional);
                let t = p.pick(step)?;
                if t.kind == Tok::Done {
                    break;
                }
                shares[t.n as usize] += 1;
                n += 1;
            }
            Ok(F::Divided(
                candidates
                    .iter()
                    .zip(shares)
                    .filter(|s| s.1 > 0)
                    .map(|(&c, s)| (c, s))
                    .collect(),
            ))
        }
        AskKind::DeclareAttackers {
            eligible,
            defenders,
            requirements,
            most,
            max_attackers,
            barred,
            alone,
            ..
        } => {
            // A defender at its cap takes no more attackers (CR 508.1c). A declaration obeys `most` requirements (CR
            // 508.1d), never by paying (Ghostly Prison: `most` counts attacks that cost nothing). A choice is offered
            // where the creatures still to come, each at its best attack (paying if need be), can make up the rest.
            // With no cap that is exact: each restriction is on one attacker at a time (`cantrip_state::attacks`). A cap
            // is room the attackers share, so there the best of each is only a bound: no legal declaration is ever
            // masked, and one the caps leave short of the most is the validator's to refuse (a global refusal). Where
            // the choices so far leave nothing that can make up the most, staying home is offered, for the validator.
            let cap = |i: usize| max_attackers.get(i).copied().unwrap_or(u8::MAX);
            let gain = |c: Ref, d: Ref| {
                requirements
                    .iter()
                    .find(|r| r.0 == c && r.1 == d)
                    .map_or(0, |r| r.2)
            };
            // An attack a restriction forbids (CR 508.1c) is never offered, and obeys nothing.
            let open = |c: Ref, d: Ref| !barred.contains(&(c, d));
            let best = |c: Ref| defenders.iter().filter(|&&d| open(c, d)).map(|&d| gain(c, d)).max().unwrap_or(0);
            let mut rest: u32 = eligible.iter().map(|&c| best(c)).sum();
            let mut obeyed = 0u32;
            let mut attacking = vec![0usize; defenders.len()];
            let mut attacks: Vec<(Ref, Ref)> = Vec::new();
            // CR 508.1c: a declaration whose one attacker is forbidden to attack alone is illegal, whatever else it
            // obeys - exactly, not as a bound. A choice that leaves such a lone attacker is offered only where a creature
            // still to come can attack something with room beside it (the ones between staying home); one attacker
            // beside it mends it, and no attack is ever taken back.
            let room = |attacking: &[usize], i: usize| cap(i) == u8::MAX || attacking[i] < usize::from(cap(i));
            let mended = |attacks: &[(Ref, Ref)], attacking: &[usize], later: &[Ref]| match attacks {
                [(a, _)] if alone.contains(a) => later
                    .iter()
                    .any(|&l| defenders.iter().enumerate().any(|(i, &d)| open(l, d) && room(attacking, i))),
                _ => true,
            };
            for (j, &c) in eligible.iter().enumerate() {
                rest -= best(c);
                let later = &eligible[j + 1..];
                let mut step = Step::default();
                step.push(Token::on(Tok::Stay, c), obeyed + rest >= *most);
                let mut lawful = vec![mended(&attacks, &attacking, later)];
                for (i, &d) in defenders.iter().enumerate() {
                    step.push(
                        Token::of(Tok::Attack, c, d, 0),
                        room(&attacking, i) && open(c, d) && obeyed + gain(c, d) + rest >= *most,
                    );
                    let mut with = attacks.clone();
                    with.push((c, d));
                    let mut now = attacking.clone();
                    now[i] += 1;
                    lawful.push(room(&attacking, i) && open(c, d) && mended(&with, &now, later));
                }
                // Lone attackers are judged before the requirements (`cantrip_state::attacks`): where nothing obeys
                // both, the most is the bound that gives way, for the validator to refuse (a global refusal).
                if step.mask.iter().zip(&lawful).any(|(&m, &l)| m && l) {
                    step.mask.iter_mut().zip(&lawful).for_each(|(m, &l)| *m &= l);
                } else if lawful.iter().any(|&l| l) {
                    step.mask = lawful;
                } else {
                    step.mask[0] = true;
                }
                let t = p.pick(step)?;
                if t.kind == Tok::Attack {
                    if let Some(i) = defenders.iter().position(|&d| d == t.b) {
                        attacking[i] += 1;
                    }
                    obeyed += gain(c, t.b);
                    attacks.push((c, t.b));
                }
            }
            Ok(F::Attack(attacks))
        }
        AskKind::DeclareBlockers {
            eligible,
            attackers,
            allowed,
            max_blockers,
            capacities,
            ..
        } => {
            let mut blocked = vec![0u8; attackers.len()];
            let mut blocks = Vec::new();
            for &c in eligible {
                // One block, or none; a creature that can block several (CR 509.1a) is asked again after each, until it
                // stops (`NoBlock`) or blocks as many as it can.
                let capacity = capacities.iter().find(|x| x.0 == c).map_or(1, |x| x.1);
                let mut made = 0u32;
                while made < capacity {
                    let mut step = Step::default();
                    step.push(Token::on(Tok::NoBlock, c), true);
                    for (i, &a) in attackers.iter().enumerate() {
                        let room = blocked[i] < max_blockers.get(i).copied().unwrap_or(u8::MAX);
                        step.push(
                            Token::of(Tok::Block, c, a, i as u32),
                            room && allowed.contains(&(c, a)) && !blocks.contains(&(c, a)),
                        );
                    }
                    let t = p.pick(step)?;
                    if t.kind != Tok::Block {
                        break;
                    }
                    blocked[t.n as usize] = blocked[t.n as usize].saturating_add(1);
                    blocks.push((c, t.b));
                    made += 1;
                }
            }
            Ok(F::Block(blocks))
        }
        AskKind::AssignCombatDamage { divisions } => {
            let mut shares = Vec::new();
            for d in divisions {
                let mut got = vec![0u32; d.creatures.len()];
                let mut to_defender = 0u32;
                for _ in 0..d.damage {
                    let lethal = got.iter().zip(&d.lethal).all(|(g, l)| g >= l);
                    let mut step = Step::default();
                    for (i, &c) in d.creatures.iter().enumerate() {
                        step.push(Token::of(Tok::Damage, d.source, c, i as u32), true);
                    }
                    step.push(
                        Token::of(Tok::Damage, d.source, Ref::None, d.creatures.len() as u32),
                        d.may_assign_to_defender && lethal,
                    );
                    if !step.mask.iter().any(|&m| m) {
                        break;
                    }
                    let t = p.pick(step)?;
                    match got.get_mut(t.n as usize) {
                        Some(g) => *g += 1,
                        None => to_defender += 1,
                    }
                }
                shares.extend(
                    d.creatures
                        .iter()
                        .zip(got)
                        .filter(|g| g.1 > 0)
                        .map(|(&c, g)| (d.source, Some(c), g)),
                );
                if to_defender > 0 {
                    shares.push((d.source, None, to_defender));
                }
            }
            Ok(F::Shares(shares))
        }
        AskKind::OrderTriggers { sources, .. } => {
            let mut order: Vec<u8> = Vec::new();
            while order.len() < sources.len() {
                let left: Vec<u8> = (0..sources.len() as u8)
                    .filter(|i| !order.contains(i))
                    .collect();
                if left.len() == 1 {
                    order.push(left[0]);
                    break;
                }
                let mut step = Step::default();
                for (i, &s) in sources.iter().enumerate() {
                    step.push(
                        Token::of(Tok::Next, s, Ref::None, i as u32),
                        !order.contains(&(i as u8)),
                    );
                }
                order.push(p.pick(step)?.n as u8);
            }
            Ok(F::Order(order))
        }
        AskKind::DistributeCards {
            cards,
            destinations,
            may_shuffle,
        } => {
            let mut placements: Vec<Vec<Ref>> = vec![Vec::new(); destinations.len()];
            let mut placed = vec![false; cards.len()];
            let mut first = true;
            while placed.iter().any(|&p| !p) {
                let left = placed.iter().filter(|&&p| !p).count();
                let short: usize = destinations
                    .iter()
                    .zip(&placements)
                    .map(|(d, p)| (d.min as usize).saturating_sub(p.len()))
                    .sum();
                let mut step = Step::default();
                if first && *may_shuffle {
                    step.push(Token::new(Tok::Shuffle), true);
                }
                for (k, &c) in cards.iter().enumerate() {
                    for (i, d) in destinations.iter().enumerate() {
                        let n = placements[i].len();
                        let room = d.max == u8::MAX || n < d.max as usize;
                        let fills = n < d.min as usize;
                        let legal = !placed[k]
                            && d.eligible.get(k).copied().unwrap_or(false)
                            && room
                            && left > short - usize::from(fills).min(short);
                        step.push(Token::of(Tok::Place, c, Ref::None, i as u32), legal);
                    }
                }
                if !step.mask.iter().any(|&m| m) {
                    // Nothing meets every destination's minimum: place anywhere the card may go.
                    for (j, t) in step.tokens.iter().enumerate() {
                        if let Some(k) = cards.iter().position(|&c| c == t.a)
                            && t.kind == Tok::Place
                        {
                            step.mask[j] = !placed[k]
                                && destinations[t.n as usize]
                                    .eligible
                                    .get(k)
                                    .copied()
                                    .unwrap_or(false);
                        }
                    }
                    if !step.mask.iter().any(|&m| m) {
                        break;
                    }
                }
                first = false;
                let t = p.pick(step)?;
                if t.kind == Tok::Shuffle {
                    return Ok(F::Placements {
                        placements: vec![Vec::new(); destinations.len()],
                        shuffle: true,
                    });
                }
                let k = (0..cards.len())
                    .find(|&k| cards[k] == t.a && !placed[k])
                    .expect("a placed card is one of the cards");
                placed[k] = true;
                placements[t.n as usize].push(t.a);
            }
            Ok(F::Placements {
                placements,
                shuffle: false,
            })
        }
    }
}

/// The machine of one request: the tokens chosen so far, and where they lead.
#[derive(Clone, Debug)]
pub struct Machine {
    ask: Ask,
    names: Vec<String>,
    chosen: Vec<Token>,
    bound: usize,
    now: Result<FactoredAnswer, Step>,
}

impl Machine {
    /// The machine for `ask`. `names` are the card names a name may be chosen among (the caller's list: a seat's own
    /// deck, the cards it has seen, a prior's); the request's own suggestion is added.
    pub fn new(ask: &Ask, names: &[String]) -> Machine {
        let mut m = Machine {
            ask: ask.clone(),
            names: names.to_vec(),
            chosen: Vec::new(),
            bound: bound(ask),
            now: Err(Step::default()),
        };
        m.now = m.run();
        m
    }

    fn run(&self) -> Result<FactoredAnswer, Step> {
        let mut p = Picker {
            chosen: &self.chosen,
            at: 0,
            cancel: cancellable(&self.ask),
        };
        match program(&self.ask, &self.names, &mut p) {
            Ok(answer) => Ok(answer),
            Err(Stop::Cancel) => Ok(FactoredAnswer::Cancel),
            Err(Stop::Ask(step)) => Err(step),
        }
    }

    /// The step to choose in, or `None` once finished.
    pub fn step(&self) -> Option<&Step> {
        self.now.as_ref().err()
    }

    /// The answer, once finished.
    pub fn answer(&self) -> Option<&FactoredAnswer> {
        self.now.as_ref().ok()
    }

    /// Choose token `i` of the current step. Panics if the machine is finished or the token is not legal.
    pub fn choose(&mut self, i: usize) {
        let step = self.step().expect("the machine has finished");
        assert!(
            step.mask[i],
            "token {i} ({:?}) is not legal",
            step.tokens[i]
        );
        self.chosen.push(step.tokens[i]);
        assert!(
            self.chosen.len() <= self.bound,
            "the machine took more than its bound of {} steps",
            self.bound
        );
        self.now = self.run();
    }

    /// Choose `token` if the current step offers it legally; whether it did.
    pub fn choose_token(&mut self, token: Token) -> bool {
        let Some(step) = self.step() else {
            return false;
        };
        match (0..step.tokens.len()).find(|&i| step.tokens[i] == token && step.mask[i]) {
            Some(i) => {
                self.choose(i);
                true
            }
            None => false,
        }
    }

    pub fn chosen(&self) -> &[Token] {
        &self.chosen
    }

    pub fn bound(&self) -> usize {
        self.bound
    }

    pub fn ask(&self) -> &Ask {
        &self.ask
    }

    /// The names a `Name` token's number is an index of.
    pub fn names(&self) -> Vec<String> {
        names_with(&self.ask, &self.names)
    }
}

/// The tokens the machine for `ask` takes to make `answer`, by the canonical rule (see the module), with `Done` taken
/// wherever the machine still asks once they run out; an error where `answer` cannot be made.
pub fn decompose(
    ask: &Ask,
    names: &[String],
    answer: &FactoredAnswer,
) -> Result<Vec<Token>, String> {
    use FactoredAnswer as F;
    let mut wanted: Vec<Token> = Vec::new();
    let mut push = |t: Token| wanted.push(t);
    match (&ask.kind, answer) {
        (AskKind::ChooseCostPayment { choices }, F::CostPayment(value)) if *value < choices.len() => {
            for shift in (0..usize::BITS / 4).rev().map(|digit| digit * 4) {
                push(Token::n(Tok::CostDigit, ((value >> shift) & 15) as u32));
            }
        }
        (AskKind::ChooseNumber { least, most }, F::Number(value)) if (*least..=*most).contains(value) => {
            let offset = value - least;
            for shift in (0..8).rev().map(|digit| digit * 4) {
                push(Token::n(Tok::X, ((offset >> shift) & 15) as u32));
            }
        }
        (_, F::Cancel) => push(Token::new(Tok::Cancel)),
        (AskKind::Priority { .. }, F::Pass) => push(Token::new(Tok::Pass)),
        (
            AskKind::Priority { options }
            | AskKind::PayCosts { options, .. }
            | AskKind::UnlessPays { options, .. },
            F::Act {
                option,
                targets,
                color,
            },
        ) if matches!(&ask.kind, AskKind::Priority { .. }) || snow_payment(&ask.kind) => {
            let o = options.get(*option as usize).ok_or("no such option")?;
            push(Token::of(Tok::Option, o.source, Ref::None, *option));
            for &t in targets {
                push(Token::on(Tok::Target, t));
            }
            if (targets.len() as u8) < o.targets_required {
                push(Token::new(Tok::Done));
            }
            if color_step(o).is_some() {
                push(Token::n(Tok::Color, color.unwrap_or(NO_COLOR)));
            }
        }
        (
            AskKind::PayCosts {
                options,
                announcing: true,
                x,
                x_most,
                ..
            },
            F::X(want),
        ) => {
            push(Token::n(
                Tok::X,
                x_bucket(
                    *x,
                    *x_most,
                    offers(options, action::INCREASE_X),
                    offers(options, action::DECREASE_X),
                    *want,
                ),
            ));
        }
        (
            AskKind::PayCosts {
                announcing: false, ..
            },
            F::AutoPay,
        ) => {
            if cancellable(ask) {
                push(Token::new(Tok::AutoPay));
            }
        }
        (AskKind::UnlessPays { .. }, F::Unless { pay: false, .. }) => {
            push(Token::new(Tok::Decline))
        }
        (
            AskKind::UnlessPays {
                options, x, x_most, ..
            },
            F::Unless { pay: true, x: want },
        ) => {
            push(Token::new(Tok::Pay));
            if let Some(now) = x {
                let up = offers(options, action::INCREASE_X);
                let down = offers(options, action::DECREASE_X);
                push(Token::n(
                    Tok::X,
                    x_bucket(*now, *x_most, up, down, want.unwrap_or(*now)),
                ));
            }
        }
        (
            AskKind::ChooseCards { .. }
            | AskKind::ChooseTarget { .. }
            | AskKind::SearchLibrary { .. }
            | AskKind::DiscardToHandSize { .. }
            | AskKind::Craft { .. }
            | AskKind::TapForPower { .. }
            | AskKind::CollectEvidence { .. },
            F::Refs(refs),
        ) => {
            for &r in refs {
                push(Token::on(Tok::Pick, r));
            }
        }
        (AskKind::ChooseModes { .. }, F::Modes(modes)) => {
            for &m in modes {
                push(Token::n(Tok::Mode, u32::from(m)));
            }
        }
        (AskKind::ChooseColors { .. }, F::Colors(colors)) => {
            for &c in colors {
                push(Token::n(Tok::Color, u32::from(c)));
            }
        }
        (AskKind::ChooseName { .. }, F::Name(None)) => push(Token::new(Tok::NoName)),
        (AskKind::ChooseName { .. }, F::Name(Some(name))) => {
            let i = names_with(ask, names)
                .iter()
                .position(|n| n == name)
                .ok_or_else(|| format!("{name} is not among the names offered"))?;
            push(Token::n(Tok::Name, i as u32));
        }
        (AskKind::ChooseSubtype { .. }, F::Subtype(s)) => push(Token::n(Tok::Subtype, *s)),
        (AskKind::May { .. } | AskKind::Mulligan { .. }, F::Yes) => push(Token::new(Tok::Yes)),
        (AskKind::May { .. } | AskKind::Mulligan { .. }, F::No) => push(Token::new(Tok::No)),
        (AskKind::ChoosePlayer { .. } | AskKind::ChooseStartingPlayer { .. }, F::Player(p)) => {
            push(Token::of(
                Tok::Player,
                Ref::Player(*p),
                Ref::None,
                u32::from(*p),
            ))
        }
        (AskKind::ChooseReplacement { sources, .. }, F::Index(i)) => push(Token::of(
            Tok::Index,
            sources.get(*i as usize).copied().ok_or("no such effect")?,
            Ref::None,
            u32::from(*i),
        )),
        (AskKind::RemoveCounters { pools, .. }, F::Counters(taken)) => {
            for (i, pool) in pools.iter().enumerate() {
                let n: u32 = taken
                    .iter()
                    .filter(|t| t.0 == pool.object && t.1 == pool.kind)
                    .map(|t| t.2)
                    .sum();
                for _ in 0..n {
                    push(Token::of(Tok::Unit, pool.object, Ref::None, i as u32));
                }
            }
        }
        (AskKind::ChooseDividedTargets { candidates, .. }, F::Divided(shares)) => {
            if shares.is_empty() {
                push(Token::new(Tok::Done));
            }
            for (i, &c) in candidates.iter().enumerate() {
                let n: u32 = shares.iter().filter(|s| s.0 == c).map(|s| s.1).sum();
                for _ in 0..n {
                    push(Token::of(Tok::Unit, c, Ref::None, i as u32));
                }
            }
        }
        (AskKind::DeclareAttackers { eligible, .. }, F::Attack(attacks)) => {
            for &c in eligible {
                match attacks.iter().find(|a| a.0 == c) {
                    Some(&(_, d)) => push(Token::of(Tok::Attack, c, d, 0)),
                    None => push(Token::on(Tok::Stay, c)),
                }
            }
        }
        (
            AskKind::DeclareBlockers {
                eligible,
                attackers,
                capacities,
                ..
            },
            F::Block(blocks),
        ) => {
            for &c in eligible {
                // Each block it makes, then a stop where it could have blocked more (CR 509.1a).
                let capacity = capacities.iter().find(|x| x.0 == c).map_or(1, |x| x.1);
                let mut made = 0u32;
                for &(_, a) in blocks.iter().filter(|b| b.0 == c) {
                    push(Token::of(
                        Tok::Block,
                        c,
                        a,
                        attackers
                            .iter()
                            .position(|&x| x == a)
                            .ok_or("no such attacker")? as u32,
                    ));
                    made += 1;
                }
                if made < capacity {
                    push(Token::on(Tok::NoBlock, c));
                }
            }
        }
        (AskKind::AssignCombatDamage { divisions }, F::Shares(shares)) => {
            for d in divisions {
                for (i, &c) in d.creatures.iter().enumerate() {
                    let n: u32 = shares
                        .iter()
                        .filter(|s| s.0 == d.source && s.1 == Some(c))
                        .map(|s| s.2)
                        .sum();
                    for _ in 0..n {
                        push(Token::of(Tok::Damage, d.source, c, i as u32));
                    }
                }
                let n: u32 = shares
                    .iter()
                    .filter(|s| s.0 == d.source && s.1.is_none())
                    .map(|s| s.2)
                    .sum();
                for _ in 0..n {
                    push(Token::of(
                        Tok::Damage,
                        d.source,
                        Ref::None,
                        d.creatures.len() as u32,
                    ));
                }
            }
        }
        (AskKind::OrderTriggers { sources, .. }, F::Order(order)) => {
            for &i in order.iter().take(order.len().saturating_sub(1)) {
                push(Token::of(
                    Tok::Next,
                    sources.get(i as usize).copied().ok_or("no such trigger")?,
                    Ref::None,
                    u32::from(i),
                ));
            }
        }
        (AskKind::DistributeCards { .. }, F::Placements { shuffle: true, .. }) => {
            push(Token::new(Tok::Shuffle))
        }
        (
            AskKind::DistributeCards { .. },
            F::Placements {
                placements,
                shuffle: false,
            },
        ) => {
            for (i, cards) in placements.iter().enumerate() {
                for &c in cards {
                    push(Token::of(Tok::Place, c, Ref::None, i as u32));
                }
            }
        }
        (kind, answer) => return Err(format!("{answer:?} is no answer to {}", kind_name(kind))),
    }
    let mut machine = Machine::new(ask, names);
    for (k, &t) in wanted.iter().enumerate() {
        if !machine.choose_token(t) {
            return Err(format!(
                "token {k} ({t:?}) is not offered legally: {:?}",
                machine.step()
            ));
        }
    }
    while machine.step().is_some() {
        if !machine.choose_token(Token::new(Tok::Done)) {
            return Err(format!(
                "the answer ran out with the machine still asking: {:?}",
                machine.step()
            ));
        }
    }
    Ok(machine.chosen.clone())
}

/// An ask's kind by name.
pub fn kind_name(kind: &AskKind) -> String {
    let debug = format!("{kind:?}");
    let end = debug.find(['{', '(', ' ']).unwrap_or(debug.len());
    debug[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_payment_indices_beyond_u16_roundtrip_without_aliases() {
        let choice = crate::CostPaymentObs { label: "Sacrifice a creature".into(), objects: Vec::new(), kind: crate::CostPaymentKindObs::Sacrifice };
        let ask = Ask { may_cancel: false, source: Ref::None, kind: AskKind::ChooseCostPayment { choices: vec![choice; 65_538] } };
        for value in [0, 255, 256, 65_535, 65_536, 65_537] {
            let answer = FactoredAnswer::CostPayment(value);
            let tokens = decompose(&ask, &[], &answer).unwrap();
            assert_eq!(tokens.len(), usize::BITS as usize / 4);
            let mut machine = Machine::new(&ask, &[]);
            for token in tokens { assert!(machine.choose_token(token)); }
            assert_eq!(machine.answer(), Some(&answer));
        }
        assert!(decompose(&ask, &[], &FactoredAnswer::CostPayment(65_538)).is_err());
    }

    #[test]
    fn wide_number_digits_roundtrip_all_endpoints_and_interior_values_exactly() {
        for (least, most) in [(0, u32::MAX), (42, u32::MAX), (u32::MAX - 300, u32::MAX), (0, 65535), (100, 500)] {
            let ask = Ask { may_cancel: false, source: Ref::None, kind: AskKind::ChooseNumber { least, most } };
            for value in [least, least + 1, least + (most - least) / 2, most] {
                let answer = FactoredAnswer::Number(value);
                let tokens = decompose(&ask, &[], &answer).unwrap();
                assert_eq!(tokens.len(), 8);
                let mut machine = Machine::new(&ask, &[]);
                for token in tokens { assert!(machine.step().unwrap().tokens.len() <= 16); assert!(machine.choose_token(token)); }
                assert_eq!(machine.answer(), Some(&answer));
            }
        }
    }
    #[test]
    fn wide_number_prefix_masks_only_remove_prefixes_with_no_legal_completion() {
        let ask = Ask { may_cancel: false, source: Ref::None, kind: AskKind::ChooseNumber { least: 100, most: 400 } };
        let mut machine = Machine::new(&ask, &[]);
        for _ in 0..5 { assert!(machine.choose_token(Token::n(Tok::X, 0))); }
        assert!(!machine.choose_token(Token::n(Tok::X, 2)));
        assert!(machine.choose_token(Token::n(Tok::X, 1)));
        assert!(!machine.choose_token(Token::n(Tok::X, 3)));
        assert!(machine.choose_token(Token::n(Tok::X, 2)));
        assert!(!machine.choose_token(Token::n(Tok::X, 13)));
        assert!(machine.choose_token(Token::n(Tok::X, 12)));
        assert_eq!(machine.answer(), Some(&FactoredAnswer::Number(400)));
        assert!(decompose(&ask, &[], &FactoredAnswer::Number(401)).is_err());
        assert!(decompose(&ask, &[], &FactoredAnswer::Number(99)).is_err());
    }
    use crate::{CounterPoolObs, DestinationObs, DivisionObs};

    fn ask(kind: AskKind) -> Ask {
        Ask {
            may_cancel: false,
            source: Ref::None,
            kind,
        }
    }

    fn o(i: u32) -> Ref {
        Ref::Object(i)
    }

    /// Choose the tokens equal to `tokens` in order; the answer.
    fn walk(ask: &Ask, tokens: &[Token]) -> Machine {
        let mut m = Machine::new(ask, &[]);
        for &t in tokens {
            assert!(
                m.choose_token(t),
                "{t:?} not offered legally in {:?}",
                m.step()
            );
        }
        m
    }

    fn legal(m: &Machine, t: Token) -> bool {
        let step = m.step().unwrap();
        step.tokens
            .iter()
            .zip(&step.mask)
            .any(|(&x, &l)| x == t && l)
    }

    fn round_trip(ask: &Ask, answer: &FactoredAnswer) {
        let tokens = decompose(ask, &[], answer).unwrap();
        assert_eq!(walk(ask, &tokens).answer(), Some(answer));
    }

    #[test]
    fn a_blocker_with_capacity_two_can_choose_two_distinct_attackers() {
        let a = ask(AskKind::DeclareBlockers {
            eligible: vec![o(1)], attackers: vec![o(2), o(3)],
            allowed: vec![(o(1), o(2)), (o(1), o(3))], required: vec![], most: 0,
            min_blockers: vec![1, 1], max_blockers: vec![255, 255], capacities: vec![(o(1), 2)],
        });
        round_trip(&a, &FactoredAnswer::Block(vec![]));
        round_trip(&a, &FactoredAnswer::Block(vec![(o(1), o(2))]));
        round_trip(&a, &FactoredAnswer::Block(vec![(o(1), o(2)), (o(1), o(3))]));
        let after_first = walk(&a, &[Token::of(Tok::Block, o(1), o(2), 0)]);
        assert!(!legal(&after_first, Token::of(Tok::Block, o(1), o(2), 0)));
        assert!(legal(&after_first, Token::of(Tok::Block, o(1), o(3), 1)));
        assert!(legal(&after_first, Token::on(Tok::NoBlock, o(1))));
        let ordinary = ask(AskKind::DeclareBlockers {
            eligible: vec![o(1)], attackers: vec![o(2), o(3)],
            allowed: vec![(o(1), o(2)), (o(1), o(3))], required: vec![], most: 0,
            min_blockers: vec![1, 1], max_blockers: vec![255, 255], capacities: vec![],
        });
        round_trip(&ordinary, &FactoredAnswer::Block(vec![(o(1), o(2))]));
        assert!(decompose(&ordinary, &[], &FactoredAnswer::Block(vec![(o(1), o(2)), (o(1), o(3))])).is_err());
    }

    #[test]
    fn blocks_mask_the_most_blockers_and_leave_menace_and_requirements_to_the_end() {
        // Two blockers, two attackers: 3 has menace (two blockers at least), 4 can't be blocked by more than one.
        let a = ask(AskKind::DeclareBlockers {
            eligible: vec![o(1), o(2)],
            attackers: vec![o(3), o(4)],
            allowed: vec![(o(1), o(3)), (o(1), o(4)), (o(2), o(3)), (o(2), o(4))],
            required: vec![(o(2), o(4))],
            most: 1,
            min_blockers: vec![2, 1],
            max_blockers: vec![255, 1],
            capacities: vec![],
        });
        let mut m = Machine::new(&a, &[]);
        assert_eq!(m.bound(), 2);
        assert!(legal(&m, Token::of(Tok::Block, o(1), o(4), 1)));
        m.choose_token(Token::of(Tok::Block, o(1), o(4), 1));
        // 4 is blocked by as many as it may be: the second blocker cannot join.
        assert!(!legal(&m, Token::of(Tok::Block, o(2), o(4), 1)));
        // Menace is the whole block's: one blocker on 3 is offered, and refused by the validator.
        assert!(legal(&m, Token::of(Tok::Block, o(2), o(3), 0)));
        m.choose_token(Token::on(Tok::NoBlock, o(2)));
        assert_eq!(m.answer(), Some(&FactoredAnswer::Block(vec![(o(1), o(4))])));
        // Both on the menace creature, decomposed blocker by blocker.
        round_trip(&a, &FactoredAnswer::Block(vec![(o(1), o(3)), (o(2), o(3))]));
        round_trip(&a, &FactoredAnswer::Block(vec![]));
        // Order in the answer is not the canonical order: decompose walks the blockers.
        let tokens = decompose(
            &a,
            &[],
            &FactoredAnswer::Block(vec![(o(2), o(4)), (o(1), o(3))]),
        )
        .unwrap();
        assert_eq!(tokens[0], Token::of(Tok::Block, o(1), o(3), 0));
    }

    /// A creature that can block any number of creatures (CR 509.1a) is asked again after each block, never the same attacker
    /// twice, until it stops; one that can block two is asked no third time.
    #[test]
    fn a_creature_that_can_block_several_is_asked_after_each_block() {
        let a = ask(AskKind::DeclareBlockers {
            eligible: vec![o(1), o(2)],
            attackers: vec![o(3), o(4), o(5)],
            allowed: vec![(o(1), o(3)), (o(1), o(4)), (o(1), o(5)), (o(2), o(3)), (o(2), o(4)), (o(2), o(5))],
            required: vec![],
            most: 0,
            min_blockers: vec![1, 1, 1],
            max_blockers: vec![255, 255, 255],
            capacities: vec![(o(1), u32::MAX), (o(2), 2)],
        });
        let mut m = Machine::new(&a, &[]);
        m.choose_token(Token::of(Tok::Block, o(1), o(4), 1));
        assert!(!legal(&m, Token::of(Tok::Block, o(1), o(4), 1)));
        m.choose_token(Token::of(Tok::Block, o(1), o(3), 0));
        m.choose_token(Token::on(Tok::NoBlock, o(1)));
        m.choose_token(Token::of(Tok::Block, o(2), o(3), 0));
        m.choose_token(Token::of(Tok::Block, o(2), o(5), 2));
        assert_eq!(m.answer(), Some(&FactoredAnswer::Block(vec![(o(1), o(4)), (o(1), o(3)), (o(2), o(3)), (o(2), o(5))])));
        round_trip(&a, &FactoredAnswer::Block(vec![(o(1), o(3)), (o(1), o(4)), (o(1), o(5)), (o(2), o(4))]));
        round_trip(&a, &FactoredAnswer::Block(vec![(o(2), o(3)), (o(2), o(4))]));
        round_trip(&a, &FactoredAnswer::Block(vec![]));
    }

    #[test]
    fn crew_offers_a_negative_power_only_while_the_total_stays_within_reach() {
        let tap = |powers: Vec<i32>, minimum| {
            ask(AskKind::TapForPower {
                purpose: "TapToPay".into(),
                cards: vec![o(1), o(2), o(3)],
                powers,
                minimum,
            })
        };
        let pick = |i| Token::on(Tok::Pick, o(i));
        // Crew 3 from powers 3, -2 and 1: the -2 creature would leave at most 2, whatever else is tapped.
        let a = tap(vec![3, -2, 1], 3);
        let m = Machine::new(&a, &[]);
        assert!(!legal(&m, pick(2)));
        let m = walk(&a, &[pick(3)]);
        assert!(!legal(&m, pick(2)));
        assert!(legal(&m, pick(1)));
        let m = walk(&a, &[pick(1)]);
        assert!(!legal(&m, pick(2)));
        assert!(legal(&m, Token::new(Tok::Done)));
        // From 3, -1 and 2 all three make 4: the -1 creature may be tapped, first or later.
        let b = tap(vec![3, -1, 2], 3);
        assert!(legal(&Machine::new(&b, &[]), pick(2)));
        round_trip(&b, &FactoredAnswer::Refs(vec![o(2), o(1), o(3)]));
        round_trip(&b, &FactoredAnswer::Refs(vec![o(1)]));
    }

    #[test]
    fn a_total_out_of_reach_from_the_start_is_cancelled_not_answered_short() {
        let cancellable = |kind| Ask {
            may_cancel: true,
            ..ask(kind)
        };
        let tap = |cards: Vec<Ref>, powers: Vec<i32>, minimum| {
            cancellable(AskKind::TapForPower {
                purpose: "TapToPay".into(),
                cards,
                powers,
                minimum,
            })
        };
        let only_cancel = |a: &Ask| {
            let m = Machine::new(a, &[]);
            let step = m.step().expect("a step");
            assert_eq!(
                step.legal().map(|i| step.tokens[i]).collect::<Vec<_>>(),
                vec![Token::new(Tok::Cancel)],
                "{step:?}"
            );
            assert_eq!(
                walk(a, &[Token::new(Tok::Cancel)]).answer(),
                Some(&FactoredAnswer::Cancel)
            );
        };
        // Teamwork 2 with one creature of power 1 left untapped once the mana was paid (CR 601.2g-h), and with none.
        only_cancel(&tap(vec![o(1)], vec![1], 2));
        only_cancel(&tap(vec![], vec![], 2));
        // A negative power takes from the total (CR 107.1b): 1 and -2 make 2 neither alone nor together.
        only_cancel(&tap(vec![o(1), o(2)], vec![1, -2], 2));
        // Collect evidence 4 from mana values 1 and 2.
        only_cancel(&cancellable(AskKind::CollectEvidence {
            cards: vec![o(1), o(2)],
            values: vec![1, 2],
            minimum: 4,
        }));
        // Within reach, every legal answer is still made in any order, and `Done` waits for the total.
        let a = tap(vec![o(1), o(2), o(3)], vec![3, -2, 1], 2);
        let m = Machine::new(&a, &[]);
        assert!(legal(&m, Token::new(Tok::Cancel)));
        assert!(!legal(&m, Token::new(Tok::Done)));
        round_trip(&a, &FactoredAnswer::Refs(vec![o(1)]));
        round_trip(&a, &FactoredAnswer::Refs(vec![o(3), o(2), o(1)]));
        round_trip(&a, &FactoredAnswer::Cancel);
        // Harmonize's "up to one" needs no total: none is an answer, with or without a creature.
        let reduce = cancellable(AskKind::TapForPower {
            purpose: "TapToReduce".into(),
            cards: vec![o(1)],
            powers: vec![0],
            minimum: 0,
        });
        round_trip(&reduce, &FactoredAnswer::Refs(vec![]));
        round_trip(&reduce, &FactoredAnswer::Refs(vec![o(1)]));
        // A request that may not be cancelled still has a step with something legal in it.
        let stuck = ask(AskKind::TapForPower {
            purpose: "TapToPay".into(),
            cards: vec![o(1)],
            powers: vec![1],
            minimum: 2,
        });
        assert!(Machine::new(&stuck, &[]).step().unwrap().legal().count() > 0);
    }

    #[test]
    fn attackers_that_must_attack_cannot_stay() {
        let a = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1), o(2)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![(o(2), Ref::Player(1), 1), (o(2), o(9), 1)],
            most: 1,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX, u8::MAX],
            barred: vec![],
            alone: vec![],
        });
        let mut m = Machine::new(&a, &[]);
        m.choose_token(Token::on(Tok::Stay, o(1)));
        assert!(!legal(&m, Token::on(Tok::Stay, o(2))));
        m.choose_token(Token::of(Tok::Attack, o(2), o(9), 0));
        assert_eq!(
            m.answer(),
            Some(&FactoredAnswer::Attack(vec![(o(2), o(9))]))
        );
    }

    #[test]
    fn attacks_mask_a_forbidden_attack_and_a_requirement_counts_only_the_attacks_left() {
        // Creature 1 may not attack player 1 (CR 508.1c: "creatures with flying can't attack you", Storm, Windrider) and
        // must attack if able (CR 508.1d): the planeswalker is its one attack, and it may not stay home.
        let a = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![(o(1), o(9), 1)],
            most: 1,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX, u8::MAX],
            barred: vec![(o(1), Ref::Player(1))],
            alone: vec![],
        });
        let m = Machine::new(&a, &[]);
        assert!(!legal(&m, Token::of(Tok::Attack, o(1), Ref::Player(1), 0)));
        assert!(!legal(&m, Token::on(Tok::Stay, o(1))));
        assert!(legal(&m, Token::of(Tok::Attack, o(1), o(9), 0)));
    }

    #[test]
    fn attacks_mask_a_full_capped_defender_and_a_required_creature_stays_where_all_are_full() {
        // Player 1 and planeswalker 9 may each be attacked by one creature at most (CR 508.1c: Judoon Enforcers, Tomik).
        let a = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1), o(2), o(3)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![(o(3), Ref::Player(1), 1), (o(3), o(9), 1)],
            most: 1,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![1, 1],
            barred: vec![],
            alone: vec![],
        });
        let mut m = Machine::new(&a, &[]);
        m.choose_token(Token::of(Tok::Attack, o(1), Ref::Player(1), 0));
        // Player 1 has its one attacker: the second creature is not offered it.
        assert!(!legal(&m, Token::of(Tok::Attack, o(2), Ref::Player(1), 0)));
        m.choose_token(Token::of(Tok::Attack, o(2), o(9), 0));
        // Both are full: the creature that must attack if able may only stay home.
        assert!(!legal(&m, Token::of(Tok::Attack, o(3), o(9), 0)));
        assert!(legal(&m, Token::on(Tok::Stay, o(3))));
        m.choose_token(Token::on(Tok::Stay, o(3)));
        assert_eq!(
            m.answer(),
            Some(&FactoredAnswer::Attack(vec![(o(1), Ref::Player(1)), (o(2), o(9))]))
        );
        round_trip(&a, &FactoredAnswer::Attack(vec![(o(3), o(9))]));
        // Where nothing caps the player, the creature that must attack may not stay.
        let open = ask(AskKind::DeclareAttackers {
            eligible: vec![o(3)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![(o(3), Ref::Player(1), 1), (o(3), o(9), 1)],
            most: 1,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX, 1],
            barred: vec![],
            alone: vec![],
        });
        assert!(!legal(&Machine::new(&open, &[]), Token::on(Tok::Stay, o(3))));
        // Where attacking that player costs something (Ghostly Prison), the requirement is not to pay (CR 508.1d): it may.
        let taxed = ask(AskKind::DeclareAttackers {
            eligible: vec![o(3)],
            defenders: vec![Ref::Player(1)],
            requirements: vec![(o(3), Ref::Player(1), 1)],
            // A requirement is never obeyed by paying: the most is 0.
            most: 0,
            costs: vec![(o(3), Ref::Player(1), 2)],
            can_pay: 0,
            max_attackers: vec![u8::MAX],
            barred: vec![],
            alone: vec![],
        });
        assert!(legal(&Machine::new(&taxed, &[]), Token::on(Tok::Stay, o(3))));
        round_trip(&taxed, &FactoredAnswer::Attack(vec![(o(3), Ref::Player(1))]));
    }

    #[test]
    fn attackers_obey_as_many_requirements_as_the_most() {
        // Told to attack player 1 and goaded by player 1 (attack, and attack a player other than 1): attacking either
        // player obeys two, the most; attacking the planeswalker one.
        let goaded = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1)],
            defenders: vec![Ref::Player(1), Ref::Player(2), o(9)],
            requirements: vec![
                (o(1), Ref::Player(1), 2),
                (o(1), Ref::Player(2), 2),
                (o(1), o(9), 1),
            ],
            most: 2,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX; 3],
            barred: vec![],
            alone: vec![],
        });
        let m = Machine::new(&goaded, &[]);
        assert!(!legal(&m, Token::on(Tok::Stay, o(1))));
        assert!(!legal(&m, Token::of(Tok::Attack, o(1), o(9), 0)));
        assert!(legal(&m, Token::of(Tok::Attack, o(1), Ref::Player(2), 0)));
        // What the creature must attack is gone (Gideon, Battle-Forged): the most is 0, and it may stay home.
        let gone = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1)],
            defenders: vec![Ref::Player(1)],
            requirements: vec![],
            most: 0,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX],
            barred: vec![],
            alone: vec![],
        });
        round_trip(&gone, &FactoredAnswer::Attack(vec![]));
        // One creature may make up for another: one paying to obey two leaves the other free to stay home.
        let paid = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1), o(2)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![
                (o(1), Ref::Player(1), 1),
                (o(2), Ref::Player(1), 1),
                (o(2), o(9), 2),
            ],
            most: 2,
            costs: vec![(o(2), o(9), 3)],
            can_pay: 3,
            max_attackers: vec![u8::MAX; 2],
            barred: vec![],
            alone: vec![],
        });
        let m = walk(&paid, &[Token::on(Tok::Stay, o(1))]);
        assert!(!legal(&m, Token::on(Tok::Stay, o(2))));
        assert!(!legal(&m, Token::of(Tok::Attack, o(2), Ref::Player(1), 0)));
        round_trip(&paid, &FactoredAnswer::Attack(vec![(o(2), o(9))]));
    }

    #[test]
    fn a_creature_that_cannot_attack_alone_attacks_only_beside_another() {
        // Creatures 1 and 2 can't attack alone (CR 508.1c: Pet Sehlat); creature 3 may attack only planeswalker 9, which
        // one creature at most may attack.
        let a = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1), o(2), o(3)],
            defenders: vec![Ref::Player(1), o(9)],
            requirements: vec![],
            most: 0,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX, 1],
            barred: vec![(o(3), Ref::Player(1))],
            alone: vec![o(1), o(2)],
        });
        // Alone, the first may attack: either other creature can join it.
        let m = walk(&a, &[Token::of(Tok::Attack, o(1), o(9), 0)]);
        // With the planeswalker taken, 3 can't join it: 2 must.
        assert!(!legal(&m, Token::on(Tok::Stay, o(2))));
        assert!(legal(&m, Token::of(Tok::Attack, o(2), Ref::Player(1), 0)));
        // Attacking the player, 3 may still join it at the planeswalker, and then must.
        let m = walk(&a, &[Token::of(Tok::Attack, o(1), Ref::Player(1), 0), Token::on(Tok::Stay, o(2))]);
        assert!(!legal(&m, Token::on(Tok::Stay, o(3))));
        assert!(legal(&m, Token::of(Tok::Attack, o(3), o(9), 0)));
        // The last creature that can't attack alone may not attack alone; nobody attacking is legal.
        let m = walk(&a, &[Token::on(Tok::Stay, o(1)), Token::on(Tok::Stay, o(2))]);
        assert!(legal(&m, Token::on(Tok::Stay, o(3))));
        let m = walk(&a, &[Token::on(Tok::Stay, o(1))]);
        // After 1 stays home, 2 attacking the planeswalker would fill the only place 3 may join it.
        assert!(!legal(&m, Token::of(Tok::Attack, o(2), o(9), 0)));
        assert!(legal(&m, Token::of(Tok::Attack, o(2), Ref::Player(1), 0)));
        let lone = ask(AskKind::DeclareAttackers {
            eligible: vec![o(1)],
            defenders: vec![Ref::Player(1)],
            requirements: vec![],
            most: 0,
            costs: vec![],
            can_pay: 0,
            max_attackers: vec![u8::MAX],
            barred: vec![],
            alone: vec![o(1)],
        });
        assert!(!legal(&Machine::new(&lone, &[]), Token::of(Tok::Attack, o(1), Ref::Player(1), 0)));
        round_trip(&a, &FactoredAnswer::Attack(vec![(o(1), Ref::Player(1)), (o(2), Ref::Player(1))]));
        round_trip(&a, &FactoredAnswer::Attack(vec![(o(2), Ref::Player(1)), (o(3), o(9))]));
    }

    #[test]
    fn divided_damage_goes_a_unit_at_a_time_within_the_most_targets() {
        let a = ask(AskKind::ChooseDividedTargets {
            candidates: vec![Ref::Player(1), o(1), o(2)],
            total: 3,
            max_targets: 2,
            optional: false,
        });
        let mut m = Machine::new(&a, &[]);
        assert!(!legal(&m, Token::new(Tok::Done)));
        m.choose_token(Token::of(Tok::Unit, o(2), Ref::None, 2));
        m.choose_token(Token::of(Tok::Unit, Ref::Player(1), Ref::None, 0));
        // Two targets named: a third is not offered, more to either is.
        assert!(!legal(&m, Token::of(Tok::Unit, o(1), Ref::None, 1)));
        m.choose_token(Token::of(Tok::Unit, o(2), Ref::None, 2));
        assert_eq!(
            m.answer(),
            Some(&FactoredAnswer::Divided(vec![
                (Ref::Player(1), 1),
                (o(2), 2)
            ]))
        );
        round_trip(
            &a,
            &FactoredAnswer::Divided(vec![(Ref::Player(1), 1), (o(2), 2)]),
        );
        let optional = ask(AskKind::ChooseDividedTargets {
            candidates: vec![o(1)],
            total: 2,
            max_targets: 0,
            optional: true,
        });
        round_trip(&optional, &FactoredAnswer::Divided(vec![]));
    }

    #[test]
    fn trample_reaches_the_player_only_past_lethal_damage() {
        // A 5-power trampler blocked by two creatures needing 2 and 1.
        let a = ask(AskKind::AssignCombatDamage {
            divisions: vec![DivisionObs {
                source: o(1),
                banding: false,
                damage: 5,
                creatures: vec![o(2), o(3)],
                lethal: vec![2, 1],
                may_assign_to_defender: true,
                defender: Ref::Player(1),
            }],
        });
        let mut m = Machine::new(&a, &[]);
        assert_eq!(m.bound(), 5);
        let to = |c: Ref, i: u32| Token::of(Tok::Damage, o(1), c, i);
        assert!(!legal(&m, to(Ref::None, 2)));
        m.choose_token(to(o(2), 0));
        m.choose_token(to(o(3), 1));
        assert!(!legal(&m, to(Ref::None, 2)));
        m.choose_token(to(o(2), 0));
        assert!(legal(&m, to(Ref::None, 2)));
        m.choose_token(to(Ref::None, 2));
        m.choose_token(to(Ref::None, 2));
        assert_eq!(
            m.answer(),
            Some(&FactoredAnswer::Shares(vec![
                (o(1), Some(o(2)), 2),
                (o(1), Some(o(3)), 1),
                (o(1), None, 2)
            ]))
        );
        round_trip(
            &a,
            &FactoredAnswer::Shares(vec![
                (o(1), Some(o(2)), 2),
                (o(1), Some(o(3)), 1),
                (o(1), None, 2),
            ]),
        );
        round_trip(&a, &FactoredAnswer::Shares(vec![(o(1), Some(o(3)), 5)]));
    }

    #[test]
    fn scry_and_surveil_place_one_card_at_a_time() {
        // Scry 2: top (in placing order) or bottom, any number of each.
        let scry = ask(AskKind::DistributeCards {
            cards: vec![o(1), o(2)],
            destinations: vec![
                DestinationObs {
                    kind: 0,
                    min: 0,
                    max: 255,
                    eligible: vec![true, true],
                },
                DestinationObs {
                    kind: 1,
                    min: 0,
                    max: 255,
                    eligible: vec![true, true],
                },
            ],
            may_shuffle: false,
        });
        let answer = FactoredAnswer::Placements {
            placements: vec![vec![o(2), o(1)], vec![]],
            shuffle: false,
        };
        round_trip(&scry, &answer);
        assert_eq!(
            decompose(&scry, &[], &answer).unwrap()[0],
            Token::of(Tok::Place, o(2), Ref::None, 0)
        );
        // One card exactly to the hand, the rest to the graveyard: the minimum is kept open to the end.
        let pick = ask(AskKind::DistributeCards {
            cards: vec![o(1), o(2), o(3)],
            destinations: vec![
                DestinationObs {
                    kind: 3,
                    min: 1,
                    max: 1,
                    eligible: vec![true, true, false],
                },
                DestinationObs {
                    kind: 2,
                    min: 0,
                    max: 255,
                    eligible: vec![true, true, true],
                },
            ],
            may_shuffle: true,
        });
        let mut m = Machine::new(&pick, &[]);
        assert!(legal(&m, Token::new(Tok::Shuffle)));
        m.choose_token(Token::of(Tok::Place, o(3), Ref::None, 1));
        m.choose_token(Token::of(Tok::Place, o(1), Ref::None, 1));
        // The last card must go to the hand.
        assert!(!legal(&m, Token::of(Tok::Place, o(2), Ref::None, 1)));
        m.choose_token(Token::of(Tok::Place, o(2), Ref::None, 0));
        assert_eq!(
            m.answer(),
            Some(&FactoredAnswer::Placements {
                placements: vec![vec![o(2)], vec![o(3), o(1)]],
                shuffle: false
            })
        );
        round_trip(
            &pick,
            &FactoredAnswer::Placements {
                placements: vec![Vec::new(), Vec::new()],
                shuffle: true,
            },
        );
    }

    #[test]
    fn x_is_a_bucket_the_seat_could_pay() {
        let options = |inc: bool| {
            let simple = |kind: u8| OptionObs {
                kind,
                source: o(1),
                index: 0,
                targets_required: 0,
                targets_optional: false,
                target_candidates: vec![],
                color_choices: 0,
                names_nothing: false,
                mana: None,
                restriction: None,
                pays_nothing: false,
                snow: false,
                pay_snow: false,
                granted_by: None,
                paid_by: None,
            };
            let mut v = vec![
                simple(action::CONFIRM_CASTING),
                simple(action::CANCEL_CASTING),
            ];
            if inc {
                v.push(simple(action::INCREASE_X));
            }
            v
        };
        let paying = |x: u32, x_most: u32, inc: bool| Ask {
            may_cancel: true,
            source: o(1),
            kind: AskKind::PayCosts {
                total: Default::default(),
                remaining: Default::default(),
                options: options(inc),
                mana_abilities: false,
                announcing: true,
                x,
                x_most,
                loyalty: false,
                attackers: vec![],
                life: 20,
            },
        };
        let a = paying(0, 13, true);
        let m = Machine::new(&a, &[]);
        let step = m.step().unwrap();
        assert_eq!(step.tokens[0], Token::new(Tok::Cancel));
        let legal: Vec<u32> = step
            .legal()
            .filter(|&i| step.tokens[i].kind == Tok::X)
            .map(|i| step.tokens[i].n)
            .collect();
        assert_eq!(legal, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12]);
        assert_eq!(
            decompose(&a, &[], &FactoredAnswer::X(14)).unwrap(),
            vec![Token::n(Tok::X, 12)]
        );
        round_trip(&a, &FactoredAnswer::X(5));
        round_trip(&a, &FactoredAnswer::Cancel);
        // Nothing more to raise it with: X is what it is.
        let none = paying(2, 13, false);
        let m = Machine::new(&none, &[]);
        let step = m.step().unwrap();
        assert_eq!(
            step.legal()
                .filter(|&i| step.tokens[i].kind == Tok::X)
                .map(|i| step.tokens[i].n)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn counters_removed_up_to_a_number_stop_anywhere_and_exactly_a_number_do_not() {
        let pool = |object, kind, count| CounterPoolObs {
            object,
            kind,
            count,
        };
        let pools = |up_to| {
            ask(AskKind::RemoveCounters {
                pools: vec![pool(o(1), 3, 1), pool(o(1), 7, 1), pool(o(2), 3, 2)],
                total: 3,
                up_to,
            })
        };
        // Glissa Sunslayer's "up to three": two counters of two kinds from one creature, and none at all.
        let a = pools(true);
        assert!(legal(&Machine::new(&a, &[]), Token::new(Tok::Done)));
        round_trip(&a, &FactoredAnswer::Counters(vec![(o(1), 3, 1), (o(1), 7, 1)]));
        round_trip(&a, &FactoredAnswer::Counters(vec![]));
        round_trip(&a, &FactoredAnswer::Counters(vec![(o(1), 7, 1), (o(2), 3, 2)]));
        // A cost removes exactly its number: no stopping short.
        let exact = pools(false);
        let m = walk(
            &exact,
            &[
                Token::of(Tok::Unit, o(1), Ref::None, 0),
                Token::of(Tok::Unit, o(1), Ref::None, 1),
            ],
        );
        assert!(!legal(&m, Token::new(Tok::Done)));
        assert!(decompose(&exact, &[], &FactoredAnswer::Counters(vec![(o(1), 3, 1)])).is_err());
        round_trip(&exact, &FactoredAnswer::Counters(vec![(o(1), 3, 1), (o(2), 3, 2)]));
    }

    #[test]
    fn cards_and_modes_stop_where_up_to_allows() {
        let a = ask(AskKind::ChooseCards {
            purpose: String::new(),
            cards: vec![o(1), o(2), o(3)],
            eligible: vec![o(1), o(2), o(3)],
            count: 2,
            up_to: true,
            total: Some(crate::TotalObs {
                values: vec![1, 2, 3],
                most: 3,
            }),
        });
        let mut m = Machine::new(&a, &[]);
        assert!(legal(&m, Token::new(Tok::Done)));
        m.choose_token(Token::on(Tok::Pick, o(2)));
        // 2 + 3 is over the total of 3.
        assert!(!legal(&m, Token::on(Tok::Pick, o(3))));
        round_trip(&a, &FactoredAnswer::Refs(vec![o(2), o(1)]));
        round_trip(&a, &FactoredAnswer::Refs(vec![]));
        let modes = ask(AskKind::ChooseModes {
            labels: vec![],
            eligible: vec![0, 1, 2],
            count: 1,
            at_most: 2,
            affordable: vec![],
            additional_cost: false,
            repeat: false,
        });
        round_trip(&modes, &FactoredAnswer::Modes(vec![2]));
        round_trip(&modes, &FactoredAnswer::Modes(vec![2, 0]));
    }
}
