//! "Target creature blocks this turn if able" (CR 509.1c, `EffectKind::MustBlock`), over test cards: one requirement, obeyed
//! by blocking any attacker the creature may block; none where the creature is tapped, can't block or may block nothing; with
//! menace, obeyed only with a second blocker and not required without one; and counted with Lure-style requirements in the
//! most a declaration must obey. Every declaration is judged by the core's validator (`BlockRequirements::judge`). Beside them, the
//! restriction "it can't be blocked this turn" (`EffectKind::CantBeBlocked`), which outlasts the attacker losing its abilities,
//! and a resolved "can't be blocked this turn except by ..." (`EffectKind::BlockRestriction`), which does too.

use crate::{Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn creature_target(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature))
}

/// Target creature blocks this turn if able.
fn provoke(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::MustBlock, Duration::EndOfTurn);
    }
    Ok(())
}

/// All creatures able to block this creature do so.
fn lures(_: &dyn Query, source: ObjectId, _: ObjectId, attacker: ObjectId) -> bool {
    attacker == source
}

static CREATURE: [TargetSpec; 1] = [target(creature_target)];
static FILLER: CardDef = CardDef { name: "Must Block Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static PROVOKE: CardDef =
    CardDef { name: "Must Block Test Provoke", types: TypeSet::of(&[CardType::Instant]), spell: spell(&CREATURE, provoke), ..CardDef::BLANK };
const fn creature(name: &'static str, keywords: Keywords) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), keywords, ..CardDef::BLANK }
}
static BEAR: CardDef = creature("Must Block Test Bear", Keywords::NONE);
static MENACE: CardDef = creature("Must Block Test Menace", Keywords::of(&[Keyword::Menace]));
static FLYER: CardDef = creature("Must Block Test Flyer", Keywords::of(&[Keyword::Flying]));
static LONE: CardDef = creature("Must Block Test Lone", Keywords::of(&[Keyword::CantBeBlockedByMoreThanOne]));
static LURE: CardDef = CardDef { block_requirements: &[lures], ..creature("Must Block Test Lure", Keywords::NONE) };
/// This creature must be blocked if able.
fn this_must_be_blocked(_: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    attacker == source
}
static PREY: CardDef = CardDef { must_be_blocked: &[this_must_be_blocked], ..creature("Must Block Test Prey", Keywords::NONE) };
static FLYING_PREY: CardDef = CardDef { must_be_blocked: &[this_must_be_blocked], ..creature("Must Block Test Flying Prey", Keywords::of(&[Keyword::Flying])) };
static MENACE_PREY: CardDef = CardDef { must_be_blocked: &[this_must_be_blocked], ..creature("Must Block Test Menace Prey", Keywords::of(&[Keyword::Menace])) };
static UNBLOCKABLE: CardDef = creature("Must Block Test Unblockable", Keywords::of(&[Keyword::CantBeBlocked]));
static HASTY: CardDef = creature("Must Block Test Hasty", Keywords::of(&[Keyword::Haste]));
/// "This creature can't be blocked this turn except by creatures with haste": the creature it is on, as an attacker.
fn except_by_haste(q: &dyn Query, on: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    on != attacker || q.chars(blocker).has(Keyword::Haste)
}
static RESTRICTING: CardDef = CardDef {
    lasting_block_rules: &[LastingBlockRule { text: "Can't be blocked except by creatures with haste", allows: except_by_haste }],
    ..creature("Must Block Test Restricting", Keywords::NONE)
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &PROVOKE, &BEAR, &MENACE, &FLYER, &LONE, &LURE, &PREY, &FLYING_PREY, &MENACE_PREY, &UNBLOCKABLE, &HASTY, &RESTRICTING])))
}

/// A game of two on turn 1, P0's deck starting with three Provokes, and `permanents` on the battlefield.
fn game(permanents: &[(&'static CardDef, PlayerId)]) -> (Game, Vec<ObjectId>) {
    let table = table();
    let filler = table.id_of(&FILLER);
    let deck = |provokes: usize| {
        let mut d: Vec<CardId> = vec![table.id_of(&PROVOKE); provokes];
        d.extend(std::iter::repeat_n(filler, 30));
        d
    };
    let mut g = Game::new(table, &Setup { decks: vec![deck(3), deck(0)], ..Default::default() });
    g.verify = true;
    let ids = permanents
        .iter()
        .map(|&(def, owner)| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            id
        })
        .collect();
    (g, ids)
}

/// Runs the game, passing and declaring nothing, until `stop` says the question asked is the one wanted.
fn until(g: &mut Game, stop: impl Fn(&Game, &Request) -> bool) -> Request {
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        if stop(g, &request) {
            return request;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the question never came");
}

/// P0 casts Provoke at each of `provoked` in the first main phase, then attacks P1 with `attackers`: the declaration of
/// blockers P1 is asked for.
fn declare(g: &mut Game, provoked: &[ObjectId], attackers: &[ObjectId]) -> Request {
    attack(g, provoked, attackers).expect("P1 is asked for blockers")
}

/// `declare`, or none where P1 has no block to declare and is not asked.
fn attack(g: &mut Game, provoked: &[ObjectId], attackers: &[ObjectId]) -> Option<Request> {
    for &creature in provoked {
        let main = until(g, |g, r| r.player == P0 && g.state.turn.step() == Step::PrecombatMain && matches!(r.kind, RequestKind::Priority { .. }));
        let RequestKind::Priority { options } = &main.kind else { unreachable!() };
        let option = options
            .iter()
            .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.state.obj(s).card == g.cards.id_of(&PROVOKE)))
            .expect("Provoke is offered");
        let mut action = Action::of(option);
        action.targets = vec![Target::Object(creature)];
        g.answer(main.id, P0, Answer::Act(action)).unwrap();
        until(g, |g, r| g.state.stack.is_empty() && r.player == P0 && matches!(r.kind, RequestKind::Priority { .. }));
    }
    let attack = until(g, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    g.answer(attack.id, P0, Answer::Attack(attackers.iter().map(|&a| (a, Target::Player(P1))).collect())).unwrap();
    let next = until(g, |g, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }) || g.state.turn.step() > Step::DeclareBlockers);
    matches!(next.kind, RequestKind::DeclareBlockers { .. }).then_some(next)
}

fn rules(request: &Request) -> &BlockRequirements {
    let RequestKind::DeclareBlockers { requirements, .. } = &request.kind else { panic!("not a declaration of blockers") };
    requirements
}

fn legal(g: &Game, request: &Request, blocks: &[(ObjectId, ObjectId)]) -> bool {
    crate::check_answer(g, request, &Answer::Block(blocks.to_vec())).is_ok()
}

/// A creature that blocks this turn if able is one requirement, obeyed by blocking either attacker; declaring no blockers, or
/// blocking with only the other creature, is refused. The block declared is made.
#[test]
fn a_creature_that_blocks_if_able_blocks_any_attacker() {
    let (mut g, ids) = game(&[(&BEAR, P0), (&BEAR, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (a1, a2, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[x], &[a1, a2]);
    assert_eq!(rules(&ask).required, vec![RequiredBlock { blocker: Some(x), attacker: None }]);
    assert_eq!(rules(&ask).most, 1);
    assert!(!legal(&g, &ask, &[]));
    assert!(!legal(&g, &ask, &[(y, a1)]));
    assert!(legal(&g, &ask, &[(x, a1)]) && legal(&g, &ask, &[(x, a2)]) && legal(&g, &ask, &[(x, a2), (y, a2)]));
    let verdict = rules(&ask).judge(allowed(&ask), &[], true);
    assert_eq!(verdict.issue.map(|i| (i.creature, i.rule)), Some((x, "509.1c")));
    g.answer(ask.id, P1, Answer::Block(vec![(x, a2)])).unwrap();
    g.run();
    assert_eq!(g.state.blocks.iter().map(|b| (b.blocker, b.attacker)).collect::<Vec<_>>(), vec![(x, a2)]);
}

fn allowed(request: &Request) -> &[(ObjectId, ObjectId)] {
    let RequestKind::DeclareBlockers { allowed, .. } = &request.kind else { panic!("not a declaration of blockers") };
    allowed
}

/// "Blocks this creature this combat if able" (CR 509.1c, 702.39a): one requirement naming the blocker and that attacker.
/// Declaring no blocks, or blocking the other attacker, is refused; blocking the named attacker, with or without a second
/// blocker, is not. With the blocker tapped, or the named attacker out of reach, nothing is required of it.
#[test]
fn a_creature_that_must_block_one_attacker_blocks_that_one() {
    for case in ["untapped", "tapped", "flyer"] {
        let named: &'static CardDef = if case == "flyer" { &FLYER } else { &BEAR };
        let (mut g, ids) = game(&[(named, P0), (&BEAR, P0), (&BEAR, P1), (&BEAR, P1)]);
        let (a1, a2, x, y) = (ids[0], ids[1], ids[2], ids[3]);
        if case == "tapped" {
            g.state.update_obj(x, |o| o.tapped = true);
        }
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect { source: a1, controller: P0, timestamp, affected: x, kind: EffectKind::MustBlockAttacker(a1), duration: Duration::EndOfCombat });
        let ask = declare(&mut g, &[], &[a1, a2]);
        if case == "untapped" {
            assert_eq!(rules(&ask).required, vec![RequiredBlock { blocker: Some(x), attacker: Some(a1) }]);
            assert!(!legal(&g, &ask, &[]) && !legal(&g, &ask, &[(x, a2)]) && !legal(&g, &ask, &[(y, a1)]));
            assert!(legal(&g, &ask, &[(x, a1)]) && legal(&g, &ask, &[(x, a1), (y, a2)]) && legal(&g, &ask, &[(x, a1), (y, a1)]));
        } else {
            assert!(rules(&ask).required.is_empty(), "{case}: the creature is required to block");
            assert!(legal(&g, &ask, &[]), "{case}: declaring no blockers is refused");
        }
    }
}

/// The requirement is not obeyed where the restrictions keep the creature from blocking (CR 509.1c, the rulings): tapped, by
/// an effect that says it can't block, or with no attacker it may block (a flyer, for a creature without flying or reach).
#[test]
fn a_creature_unable_to_block_is_required_nothing() {
    for case in ["tapped", "can't block", "flyer"] {
        let attacker: &'static CardDef = if case == "flyer" { &FLYER } else { &BEAR };
        let (mut g, ids) = game(&[(attacker, P0), (&BEAR, P1), (&BEAR, P1)]);
        let (a, x) = (ids[0], ids[1]);
        match case {
            "tapped" => g.state.update_obj(x, |o| o.tapped = true),
            "can't block" => {
                let timestamp = g.state.timestamp();
                g.state.effect_push(ContinuousEffect { source: x, controller: P0, timestamp, affected: x, kind: EffectKind::CantBlock, duration: Duration::EndOfTurn });
            }
            _ => {}
        }
        // The requirement is weighed as blockers are declared, whatever the creature was when Provoke resolved. With no
        // block possible P1 is not asked at all; otherwise nothing is required of the creature.
        if let Some(ask) = attack(&mut g, &[x], &[a]) {
            assert_eq!(rules(&ask).most, 0, "{case}: something is required");
            assert!(legal(&g, &ask, &[]), "{case}: declaring no blockers is refused");
            assert!(rules(&ask).required.is_empty(), "{case}: the creature is required to block");
        } else {
            assert_ne!(case, "can't block", "the other Bear could block");
        }
        assert!(g.state.blocks.is_empty() || case == "can't block");
    }
}

/// With menace, the provoked creature obeys only alongside a second blocker: with one able to join it, both block or the
/// declaration is refused; the creature alone may not block, and with no second creature nothing is required.
#[test]
fn a_menace_attacker_needs_a_second_blocker_for_the_requirement() {
    let (mut g, ids) = game(&[(&MENACE, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (m, x, y) = (ids[0], ids[1], ids[2]);
    let ask = declare(&mut g, &[x], &[m]);
    assert_eq!(rules(&ask).most, 1);
    assert!(!legal(&g, &ask, &[]) && !legal(&g, &ask, &[(x, m)]));
    assert!(legal(&g, &ask, &[(x, m), (y, m)]));
    let one = rules(&ask).judge(allowed(&ask), &[(x, m)], true);
    assert_eq!(one.addable, vec![(y, m)], "the second blocker is the one block that completes it");

    // With no second creature the menace attacker can't be blocked at all, so P1 is not asked, and no block is made.
    let (mut g, ids) = game(&[(&MENACE, P0), (&BEAR, P1)]);
    let (m, x) = (ids[0], ids[1]);
    assert!(attack(&mut g, &[x], &[m]).is_none());
    assert!(g.state.blocks.is_empty());
}

/// A Lure-style attacker and a provoked creature: X must block (any attacker), and X and Y must each block the lured
/// creature. X and Y both on it obey all three; X on the other attacker obeys one of X's, and is refused.
#[test]
fn a_requirement_to_block_counts_with_lure_requirements() {
    let (mut g, ids) = game(&[(&LURE, P0), (&BEAR, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (l, a, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[x], &[l, a]);
    assert_eq!(rules(&ask).required.len(), 3);
    assert_eq!(rules(&ask).most, 3);
    assert!(legal(&g, &ask, &[(x, l), (y, l)]));
    assert!(!legal(&g, &ask, &[(x, a), (y, l)]));
    assert!(!legal(&g, &ask, &[(y, l)]));
}

/// "This creature must be blocked if able" (CR 509.1c): one requirement on the attacker, obeyed by any creature blocking it. Blocking
/// only the other attacker, or nothing, is refused; one blocker, either one, on the creature obeys it. Two such attackers are two
/// requirements, and blocking one of them obeys one.
#[test]
fn a_creature_that_must_be_blocked_is_blocked_by_any_creature_able() {
    let (mut g, ids) = game(&[(&PREY, P0), (&BEAR, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (prey, a, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[], &[prey, a]);
    assert_eq!(rules(&ask).required, vec![RequiredBlock { blocker: None, attacker: Some(prey) }]);
    assert_eq!(rules(&ask).most, 1);
    assert!(!legal(&g, &ask, &[]) && !legal(&g, &ask, &[(x, a)]) && !legal(&g, &ask, &[(x, a), (y, a)]));
    assert!(legal(&g, &ask, &[(x, prey)]) && legal(&g, &ask, &[(y, prey), (x, a)]));
    let (mut g, ids) = game(&[(&PREY, P0), (&PREY, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (p, q, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[], &[p, q]);
    assert_eq!(rules(&ask).required.len(), 2);
    assert_eq!(rules(&ask).most, 2);
    assert!(!legal(&g, &ask, &[(x, p)]) && !legal(&g, &ask, &[(x, p), (y, p)]));
    assert!(legal(&g, &ask, &[(x, p), (y, q)]));
}

/// The requirement is obeyed only as far as the restrictions allow (CR 509.1c): an attacker with flying that nothing able to block
/// it can be blocked by needs no block: the defending player has none to declare and is not asked.
#[test]
fn a_creature_that_must_be_blocked_but_cannot_be_is_not_required_a_block() {
    let (mut g, ids) = game(&[(&FLYING_PREY, P0), (&BEAR, P1)]);
    assert!(attack(&mut g, &[], &[ids[0]]).is_none());
    assert!(g.state.blocks.is_empty());
}

/// Two provoked creatures and two attackers: each creature is a requirement of its own, obeyed only where both block.
#[test]
fn each_provoked_creature_is_its_own_requirement() {
    let (mut g, ids) = game(&[(&BEAR, P0), (&BEAR, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (a1, a2, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[x, y], &[a1, a2]);
    assert_eq!(rules(&ask).most, 2);
    assert!(!legal(&g, &ask, &[(x, a1)]));
    assert!(legal(&g, &ask, &[(x, a1), (y, a1)]) && legal(&g, &ask, &[(x, a1), (y, a2)]));
}

/// CR 509.1c counts requirements, one per effect: X provoked twice and Y once, attacked by a creature only one creature may
/// block. X blocking obeys two, Y one: only X's block is legal.
#[test]
fn two_effects_on_one_creature_are_two_requirements() {
    let (mut g, ids) = game(&[(&LONE, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (a, x, y) = (ids[0], ids[1], ids[2]);
    let ask = declare(&mut g, &[x, x, y], &[a]);
    assert_eq!(rules(&ask).required.len(), 3);
    assert_eq!(rules(&ask).most, 2);
    assert!(legal(&g, &ask, &[(x, a)]));
    assert!(!legal(&g, &ask, &[(y, a)]));
}

/// A resolved "it can't be blocked this turn" (`EffectKind::CantBeBlocked`) is an effect on the creature, not an ability it
/// gains (CR 113.1a, 611.2c), so a later "loses all abilities" (CR 613.1f: Turn to Frog) leaves it: no block of that attacker
/// is offered. The printed "can't be blocked" (`Keyword::CantBeBlocked`) is an ability, and the same loss takes it.
#[test]
fn a_resolved_cant_be_blocked_outlasts_losing_all_abilities_and_a_printed_one_does_not() {
    let (mut g, ids) = game(&[(&BEAR, P0), (&UNBLOCKABLE, P0), (&BEAR, P1)]);
    let (effect, printed, x) = (ids[0], ids[1], ids[2]);
    let push = |g: &mut Game, affected: ObjectId, kind: EffectKind| {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect { source: affected, controller: P0, timestamp, affected, kind, duration: Duration::EndOfTurn });
    };
    push(&mut g, effect, EffectKind::CantBeBlocked);
    push(&mut g, effect, EffectKind::LoseAllAbilities);
    push(&mut g, printed, EffectKind::LoseAllAbilities);
    let ask = declare(&mut g, &[], &[effect, printed]);
    assert_eq!(allowed(&ask), &[(x, printed)]);
    assert!(!legal(&g, &ask, &[(x, effect)]));
}

/// A resolved "it can't be blocked this turn except by creatures with haste" (`EffectKind::BlockRestriction`, Gingerbrute's)
/// is an effect, not an ability (CR 113.1a, 611.2c): after the creature loses all abilities (CR 613.1f) only the hasty
/// creature is offered to block it, while a creature it is not on may still be blocked by either.
#[test]
fn a_resolved_block_restriction_outlasts_losing_all_abilities_and_is_only_on_its_creature() {
    let (mut g, ids) = game(&[(&RESTRICTING, P0), (&BEAR, P0), (&BEAR, P1), (&HASTY, P1)]);
    let (restricted, other, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let card = g.cards.id_of(&RESTRICTING);
    let push = |g: &mut Game, affected: ObjectId, kind: EffectKind| {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect { source: affected, controller: P0, timestamp, affected, kind, duration: Duration::EndOfTurn });
    };
    push(&mut g, restricted, EffectKind::BlockRestriction { card, index: 0 });
    push(&mut g, restricted, EffectKind::LoseAllAbilities);
    let ask = declare(&mut g, &[], &[restricted, other]);
    let mut pairs = allowed(&ask).to_vec();
    pairs.sort();
    let mut expected = vec![(x, other), (y, restricted), (y, other)];
    expected.sort();
    assert_eq!(pairs, expected);
    assert!(!legal(&g, &ask, &[(x, restricted)]));
    assert!(legal(&g, &ask, &[(y, restricted)]));
}

/// "Must be blocked if able" with menace (CR 509.1c, 702.110b): the attacker can be blocked only by two creatures, so the requirement is
/// obeyed by both blocking it and one blocker alone is refused; with a single creature able to block, it can't be blocked and nothing is
/// required.
#[test]
fn a_creature_that_must_be_blocked_with_menace_needs_two_blockers() {
    let (mut g, ids) = game(&[(&MENACE_PREY, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (prey, x, y) = (ids[0], ids[1], ids[2]);
    let ask = declare(&mut g, &[], &[prey]);
    assert_eq!(rules(&ask).most, 1);
    assert!(!legal(&g, &ask, &[]) && !legal(&g, &ask, &[(x, prey)]));
    assert!(legal(&g, &ask, &[(x, prey), (y, prey)]));
    let (mut g, ids) = game(&[(&MENACE_PREY, P0), (&BEAR, P1)]);
    assert!(attack(&mut g, &[], &[ids[0]]).is_none());
    assert!(g.state.blocks.is_empty());
}
