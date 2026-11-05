//! CR 509.3c distinguishes becoming blocked from each blocking relationship. Rampage (702.23) uses that transition
//! and the blockers present at resolution. False Orders and Imprison additionally consult this combat's history.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::{combat, rampage};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static FILLER: CardDef = CardDef { name: "Blocked transition filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static ATTACKER: CardDef = CardDef {
    name: "Blocked transition rampager", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(8),
    abilities: &[rampage::trigger::<2>("Rampage 2")], ..CardDef::BLANK
};
static TWICE: CardDef = CardDef {
    name: "Blocked transition double rampager",
    abilities: &[rampage::trigger::<1>("Rampage 1"), rampage::trigger::<2>("Rampage 2")], ..ATTACKER
};
static BANDED: CardDef = CardDef { name: "Blocked transition banding rampager", keywords: Keywords::of(&[Keyword::Banding]), ..ATTACKER };
fn remove_self(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.remove_from_combat(item.object);
    Ok(())
}
fn remove_self_and_unblock(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    combat::remove_from_combat_and_unblock(g, item.object);
    Ok(())
}
static MULTI_BLOCKER: CardDef = CardDef {
    name: "Blocked transition multi blocker", types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(10),
    abilities: &[block_allowance(1, "This creature can block an additional creature each combat.")], ..CardDef::BLANK
};
static BLOCKER: CardDef = CardDef {
    name: "Blocked transition blocker", types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(10),
    abilities: &[ability(Cost::NONE, remove_self, "{0}: Remove this creature from combat."),
                 ability(Cost::NONE, remove_self_and_unblock, "{0}: Remove this creature from combat and unblock qualifying attackers.")],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), owner, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn game(def: &'static CardDef, count: usize) -> (Game, ObjectId, Vec<ObjectId>) {
    let table = Box::leak(Box::new(CardTable::new([&FILLER, &ATTACKER, &TWICE, &BANDED, &BLOCKER, &MULTI_BLOCKER])));
    let deck = vec![table.id_of(&FILLER); 30];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    game.verify = true;
    let attacker = put(&mut game, def, YOU);
    let blockers = (0..count).map(|_| put(&mut game, &BLOCKER, THEM)).collect();
    (game, attacker, blockers)
}

fn until(game: &mut Game, stop: impl Fn(&Game, &Request) -> bool) -> Request {
    for _ in 0..300 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        if stop(game, &request) { return request; }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..sources.len() as u32).collect()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("question not reached");
}

fn attack(game: &mut Game, attackers: &[ObjectId]) -> Request {
    let request = until(game, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    game.answer(request.id, YOU, Answer::Attack(attackers.iter().map(|&a| (a, Target::Player(THEM))).collect())).unwrap();
    until(game, |_, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }))
}

fn block(game: &mut Game, request: Request, blocks: &[(ObjectId, ObjectId)]) {
    game.answer(request.id, THEM, Answer::Block(blocks.to_vec())).unwrap();
    until(game, |_, r| matches!(r.kind, RequestKind::Priority { .. }));
}

fn resolved(game: &mut Game) {
    until(game, |g, r| g.state.stack.is_empty() && matches!(r.kind, RequestKind::Priority { .. }));
}

/// Resolve the blocker's real activated ability above any pending rampage triggers.
fn remove_blocker(game: &mut Game, blocker: ObjectId, unblocking: bool) {
    let request = until(game, |_, r| r.player == THEM && matches!(r.kind, RequestKind::Priority { .. }));
    let RequestKind::Priority { options, .. } = &request.kind else { unreachable!() };
    let index = u8::from(unblocking);
    let action = options.iter().find(|a| a.source == Some(blocker) && a.kind == ActionKind::ActivateAbility && a.index == index).expect("removal offered");
    let remaining = game.state.stack.len();
    game.answer(request.id, THEM, Answer::Act(Action::of(action))).unwrap();
    until(game, |g, r| g.state.stack.len() == remaining && matches!(r.kind, RequestKind::Priority { .. }));
}

fn transitions(game: &Game, attacker: ObjectId) -> usize {
    game.log.iter().filter(|r| matches!(r.event, Event::BecameBlocked { attacker: a } if a == attacker)).count()
}

#[test]
fn three_blockers_make_one_transition_and_one_rampage_trigger() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 3);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &blockers.iter().map(|&b| (b, attacker)).collect::<Vec<_>>());
    assert_eq!(transitions(&game, attacker), 1);
    assert_eq!(game.state.stack.len(), 1);
    resolved(&mut game);
    assert_eq!((game.chars(attacker).power, game.chars(attacker).toughness), (Some(6), Some(12)));
}

#[test]
fn a_single_blocker_triggers_but_gives_no_rampage_bonus() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 1);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &[(blockers[0], attacker)]);
    assert_eq!(game.state.stack.len(), 1);
    resolved(&mut game);
    assert_eq!(game.chars(attacker).power, Some(2));
}

#[test]
fn rampage_counts_blockers_at_resolution_and_then_keeps_that_bonus() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 3);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &blockers.iter().map(|&b| (b, attacker)).collect::<Vec<_>>());
    remove_blocker(&mut game, blockers[0], false);
    assert!(game.is_blocked(attacker));
    resolved(&mut game);
    assert_eq!(game.chars(attacker).power, Some(4));
    remove_blocker(&mut game, blockers[1], false);
    assert_eq!(game.chars(attacker).power, Some(4));
}

#[test]
fn multiple_rampage_instances_trigger_independently() {
    let (mut game, attacker, blockers) = game(&TWICE, 3);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &blockers.iter().map(|&b| (b, attacker)).collect::<Vec<_>>());
    assert_eq!(transitions(&game, attacker), 1);
    assert_eq!(game.state.stack.len(), 2);
    resolved(&mut game);
    assert_eq!(game.chars(attacker).power, Some(8));
}

#[test]
fn an_effect_can_block_an_attacker_without_creating_a_blocker() {
    let (mut game, attacker, _) = game(&ATTACKER, 0);
    let other = put(&mut game, &BLOCKER, THEM);
    // An explicit position: an attacking creature, before a blocker has been declared.
    game.state.set_attackers(vec![Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE)]);
    for id in [attacker, attacker, other] { cantrip_api::Game::become_blocked(&mut cx(&mut game), id); }
    assert!(game.is_blocked(attacker));
    assert_eq!(transitions(&game, attacker), 1);
    assert!(game.state.blocks.is_empty());
    assert_eq!(game.state.triggers.len(), 1);
    cantrip_api::Game::become_unblocked(&mut cx(&mut game), attacker);
    cantrip_api::Game::become_blocked(&mut cx(&mut game), attacker);
    assert_eq!(transitions(&game, attacker), 2);
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::Unblocked { .. })));
}

#[test]
fn ordinary_removal_leaves_an_attacker_blocked_but_the_printed_unblocking_recipe_does_not() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 1);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &[(blockers[0], attacker)]);
    remove_blocker(&mut game, blockers[0], true);
    assert!(!game.is_blocked(attacker));
    assert!(game.state.blocks.is_empty());
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::Unblocked { .. })));
}

#[test]
fn having_had_a_second_blocker_prevents_false_orders_unblocking_the_attacker() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 2);
    let request = attack(&mut game, &[attacker]);
    block(&mut game, request, &[(blockers[0], attacker), (blockers[1], attacker)]);
    remove_blocker(&mut game, blockers[0], false);
    remove_blocker(&mut game, blockers[1], true);
    assert!(game.is_blocked(attacker));
    assert!(game.state.blocks.is_empty());
}

#[test]
fn a_banded_attacker_also_becomes_blocked_once() {
    let (mut game, attacker, blockers) = game(&BANDED, 2);
    let other = put(&mut game, &ATTACKER, YOU);
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    game.answer(request.id, YOU, Answer::Attack(vec![(attacker, Target::Player(THEM)), (other, Target::Player(THEM))])).unwrap();
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Band, .. }));
    game.answer(request.id, YOU, Answer::Cards(vec![other])).unwrap();
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }));
    block(&mut game, request, &[(blockers[0], attacker), (blockers[1], attacker)]);
    assert_eq!(transitions(&game, attacker), 1);
    assert_eq!(transitions(&game, other), 1);
    assert_eq!(game.state.stack.len(), 2);
}

#[test]
fn search_can_undo_and_redo_the_blocked_transition_and_its_trigger() {
    let (mut game, attacker, blockers) = game(&ATTACKER, 1);
    let request = attack(&mut game, &[attacker]);
    let before = game.at;
    block(&mut game, request, &[(blockers[0], attacker)]);
    let blocked = game.at;
    let state = game.state.clone();
    let history = game.log.clone();
    game.goto(before);
    assert!(!game.is_blocked(attacker));
    assert_eq!(transitions(&game, attacker), 0);
    assert!(game.state.stack.is_empty());
    game.goto(blocked);
    assert_eq!(game.state, state);
    assert_eq!(game.log, history);
}

#[test]
fn selecting_two_members_of_a_band_makes_each_block_only_once() {
    let (mut game, attacker, _) = game(&BANDED, 0);
    let other = put(&mut game, &ATTACKER, YOU);
    let blocker = put(&mut game, &MULTI_BLOCKER, THEM);
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    game.answer(request.id, YOU, Answer::Attack(vec![(attacker, Target::Player(THEM)), (other, Target::Player(THEM))])).unwrap();
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Band, .. }));
    game.answer(request.id, YOU, Answer::Cards(vec![other])).unwrap();
    let request = until(&mut game, |_, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }));
    block(&mut game, request, &[(blocker, attacker), (blocker, other)]);
    assert_eq!(game.state.blocks.iter().filter(|b| b.blocker == blocker).count(), 2);
    let events: Vec<_> = game.log.iter().filter_map(|r| match r.event { Event::Blocks { blocker: b, attacker, declared } if b == blocker => Some((attacker, declared)), _ => None }).collect();
    assert_eq!(events, vec![(attacker, true), (other, false)]);
}
