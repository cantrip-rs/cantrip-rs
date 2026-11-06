//! Blocking status outlives its attacker associations (CR 509.1g, 510.1d).
use super::*;
use cantrip_api::Game as Rules;
use std::hash::{Hash, Hasher};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static CREATURE: CardDef = CardDef {
    name: "Blocking status creature", types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2), ..CardDef::BLANK
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn game() -> (Game, ObjectId, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE])));
    let deck = vec![table.id_of(&CREATURE); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Settle));
    g.state.update_turn(|t| { t.active = YOU; t.number = 1; t.set_step(Step::DeclareBlockers); });
    let mut put = |owner| {
        let id = g.state.push_obj(Obj::new(table.id_of(&CREATURE), owner, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id); id
    };
    let attacker = put(YOU); let blocker = put(THEM);
    let mut attacking = Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE); attacking.blocked = true;
    g.state.set_attackers(vec![attacking]);
    g.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
    (g, attacker, blocker)
}

#[test]
fn removing_an_attacker_preserves_its_surviving_blockers_status() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    assert!(g.state.blocks.is_empty());
    assert!(cx(&mut g).is_blocking(blocker));
}

#[test]
fn returning_the_attacker_to_hand_preserves_its_surviving_blockers_status() {
    let (mut g, attacker, blocker) = game();
    cx(&mut g).return_to_hand(&[attacker]);
    assert!(g.state.blocks.is_empty());
    assert!(cx(&mut g).is_blocking(blocker));
}

#[test]
fn ending_combat_clears_even_a_blocker_whose_attacker_has_left() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    assert!(cx(&mut g).is_blocking(blocker));
    g.state.update_turn(|t| t.set_step(Step::EndOfCombat));
    crate::turn::next_step(&mut g);
    assert!(!cx(&mut g).is_blocking(blocker));
}

#[test]
fn removing_a_blocker_preserves_its_attackers_blocked_status() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[blocker]);
    assert!(!cx(&mut g).is_blocking(blocker));
    assert!(cx(&mut g).is_blocked(attacker));
}

#[test]
fn regenerating_a_blocker_removes_it_even_when_its_attacker_is_gone() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    assert_eq!(ops::regenerate_instead(&mut cx(&mut g), &[(blocker, false)]), vec![blocker]);
    assert!(!cx(&mut g).is_blocking(blocker));
}

#[test]
fn a_surviving_blocker_that_stops_being_a_creature_leaves_combat_for_good() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    cx(&mut g).add_effect(blocker, blocker, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::EndOfTurn);
    assert!(!cx(&mut g).is_blocking(blocker));
    cx(&mut g).add_effect(blocker, blocker, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
    assert!(cx(&mut g).chars(blocker).is(CardType::Creature));
    assert!(!cx(&mut g).is_blocking(blocker));
}

#[test]
fn an_attacker_that_stops_being_a_creature_leaves_its_blocker_blocking() {
    let (mut g, attacker, blocker) = game();
    cx(&mut g).add_effect(attacker, attacker, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::EndOfTurn);
    assert!(!cx(&mut g).is_attacking(attacker));
    assert!(g.state.blocks.is_empty());
    assert!(cx(&mut g).is_blocking(blocker));
}

#[test]
fn a_surviving_blocker_that_changes_controller_leaves_combat() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    cx(&mut g).add_effect(blocker, blocker, EffectKind::Control(YOU), Duration::EndOfTurn);
    assert_eq!(cx(&mut g).controller(blocker), YOU);
    assert!(!cx(&mut g).is_blocking(blocker));
}

#[test]
fn a_surviving_blocker_that_leaves_the_battlefield_stops_blocking() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    cx(&mut g).return_to_hand(&[blocker]);
    assert!(!cx(&mut g).is_blocking(blocker));
}

#[test]
fn ending_the_turn_during_combat_also_clears_the_surviving_blocker() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    g.state.update_turn(|t| t.ending = true);
    crate::turn::next_step(&mut g);
    assert!(!cx(&mut g).is_blocking(blocker));
}

fn state_hash(g: &Game) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    g.state.hash(&mut hash); hash.finish()
}

#[test]
fn blocking_status_and_relationships_rollback_and_replay_together() {
    let (mut g, attacker, blocker) = game();
    let mark = g.state.mark(); let before = state_hash(&g);
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    let after = state_hash(&g); assert!(cx(&mut g).is_blocking(blocker));
    g.state.rollback(mark); assert_eq!(state_hash(&g), before);
    assert_eq!(g.state.blocks.len(), 1); assert!(cx(&mut g).is_blocking(blocker));
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    assert_eq!(state_hash(&g), after);
}

#[test]
fn loop_detection_distinguishes_a_surviving_blocker_from_a_nonblocking_creature() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    let blocking = crate::loops::position(&g.state); let hash = state_hash(&g);
    g.state.set_blockers(Vec::new());
    assert!(!cx(&mut g).is_blocking(blocker));
    assert_ne!(crate::loops::position(&g.state), blocking);
    assert_ne!(state_hash(&g), hash);
}

#[test]
fn multiple_attacker_associations_still_have_one_blocking_creature() {
    let (mut g, attacker, blocker) = game();
    let other = g.state.push_obj(Obj::new(g.cards.id_of(&CREATURE), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, other);
    g.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }, Block { blocker, attacker: other, first_strike: false }]);
    assert_eq!(g.state.blockers, vec![blocker]);
    ops::remove_from_combat(&mut cx(&mut g), &[attacker, other]);
    assert!(g.state.blocks.is_empty()); assert_eq!(g.state.blockers, vec![blocker]);
}

#[test]
fn a_blocker_without_remaining_attackers_assigns_no_combat_damage() {
    let (mut g, attacker, blocker) = game();
    ops::remove_from_combat(&mut cx(&mut g), &[attacker]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    crate::turn::begin_step(&mut cx(&mut g)).unwrap();
    assert!(cx(&mut g).is_blocking(blocker));
    assert_eq!(g.state.turn.damage_round, 2, "an ordinary blocker creates no first-strike step");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::Damage { kind, .. } if kind.is_combat())));
}

#[test]
fn a_first_strike_or_double_strike_blocker_without_an_attacker_still_creates_both_damage_steps() {
    for keyword in [Keyword::FirstStrike, Keyword::DoubleStrike] {
        let (mut g, attacker, blocker) = game();
        cx(&mut g).add_effect(blocker, blocker, EffectKind::AddKeywords(Keywords::of(&[keyword])), Duration::EndOfTurn);
        cx(&mut g).return_to_hand(&[attacker]);
        g.state.update_turn(|t| t.set_step(Step::CombatDamage));
        crate::turn::begin_step(&mut cx(&mut g)).unwrap();
        assert_eq!(g.state.turn.damage_round, 1, "{keyword:?}: the first-strike damage step exists");
        assert!(g.state.agenda.iter().any(|frame| matches!(frame.kind, cantrip_state::FrameKind::Priority { .. })), "priority is offered between the two damage steps");
        crate::turn::next_step(&mut g);
        assert_eq!(g.state.turn.step(), Step::CombatDamage);
        crate::turn::begin_step(&mut cx(&mut g)).unwrap();
        assert_eq!(g.state.turn.damage_round, 2);
        assert!(!g.log.iter().any(|r| matches!(r.event, Event::Damage { kind, .. } if kind.is_combat())));
    }
}
