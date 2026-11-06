//! A specified next turn's step differs from the next occurrence of a step (CR 603.7, 614.10a).
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
use cantrip_api::Game as Rules;
use std::hash::{Hash, Hasher};

fn state_hash(g: &Game) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    g.state.hash(&mut hash); hash.finish()
}

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static SOURCE: CardDef = CardDef {
    name: "Next turn delay source", types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[delayed_trigger(nothing, "Draw a card at the beginning of the next turn's upkeep.")],
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Next turn delay filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn game() -> (Game, ObjectId) {
    game_with(2)
}
fn game_with(seats: usize) -> (Game, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&SOURCE, &FILLER])));
    let deck = vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck; seats], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let id = g.state.push_obj(Obj::new(table.id_of(&SOURCE), YOU, Zone::graveyard(YOU)));
    g.state.zone_push(Zone::graveyard(YOU), id);
    turn(&mut g, YOU, 10, false);
    (g, id)
}
fn turn(g: &mut Game, player: PlayerId, number: u32, extra: bool) {
    g.state.update_turn(|t| { t.active = player; t.number = number; t.set_step(Step::Untap); });
    cx(g).record(Event::TurnBegan { player, extra });
}
fn schedule(g: &mut Game, source: ObjectId, of: Option<PlayerId>) {
    let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    cx(g).delayed_trigger(DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of }, source, YOU, ability, Target::None);
}
fn step(g: &mut Game, at: Step) {
    g.state.update_turn(|t| t.set_step(at));
    crate::settle::step_began(&mut cx(g), at);
}
fn cleanup(g: &mut Game) {
    g.state.update_turn(|t| t.set_step(Step::Cleanup));
    crate::turn::cleanup(&mut cx(g)).expect("no cleanup question");
}

#[test]
fn an_extra_upkeep_of_the_creation_turn_is_not_the_next_turns_upkeep() {
    let (mut g, source) = game(); schedule(&mut g, source, None);
    step(&mut g, Step::Upkeep);
    assert!(g.state.triggers.is_empty()); assert_eq!(g.state.delayed.len(), 1);
}

#[test]
fn an_extra_turn_is_the_next_turn_even_when_the_same_player_takes_it() {
    let (mut g, source) = game(); schedule(&mut g, source, None);
    turn(&mut g, YOU, 11, true); step(&mut g, Step::Upkeep);
    assert_eq!(g.state.triggers.len(), 1); assert!(g.state.delayed.is_empty());
    let trigger = g.state.triggers[0]; assert_eq!(trigger.source, source); assert_eq!(trigger.controller, YOU);
    step(&mut g, Step::Upkeep); assert_eq!(g.state.triggers.len(), 1);
}

#[test]
fn a_skipped_next_turn_upkeep_does_not_wait_for_a_later_turns_upkeep() {
    let (mut g, source) = game(); schedule(&mut g, source, None);
    turn(&mut g, THEM, 11, false); step(&mut g, Step::PrecombatMain); cleanup(&mut g);
    assert!(g.state.delayed.is_empty());
    turn(&mut g, YOU, 12, false); step(&mut g, Step::Upkeep); assert!(g.state.triggers.is_empty());
}

#[test]
fn a_later_upkeep_in_the_specified_turn_can_still_trigger_when_the_earlier_one_was_skipped() {
    let (mut g, source) = game(); schedule(&mut g, source, None);
    turn(&mut g, THEM, 11, false); step(&mut g, Step::PrecombatMain); step(&mut g, Step::Upkeep);
    assert_eq!(g.state.triggers.len(), 1); assert!(g.state.delayed.is_empty());
}

#[test]
fn a_players_next_turn_ignores_other_players_turns_and_current_turn_extra_upkeeps() {
    let (mut g, source) = game(); schedule(&mut g, source, Some(YOU));
    step(&mut g, Step::Upkeep); assert!(g.state.triggers.is_empty());
    turn(&mut g, THEM, 11, false); step(&mut g, Step::Upkeep); cleanup(&mut g);
    assert!(g.state.triggers.is_empty()); assert_eq!(g.state.delayed.len(), 1);
    turn(&mut g, YOU, 12, false); step(&mut g, Step::Upkeep); assert_eq!(g.state.triggers.len(), 1);
}

#[test]
fn a_specified_players_skipped_upkeep_expires_at_that_turns_cleanup() {
    let (mut g, source) = game(); schedule(&mut g, source, Some(YOU));
    turn(&mut g, THEM, 11, false); cleanup(&mut g); assert_eq!(g.state.delayed.len(), 1);
    turn(&mut g, YOU, 12, false); step(&mut g, Step::PrecombatMain); cleanup(&mut g);
    assert!(g.state.delayed.is_empty()); assert!(g.state.triggers.is_empty());
}

#[test]
fn creation_during_the_selected_players_turn_waits_for_their_subsequent_turn() {
    let (mut g, source) = game(); schedule(&mut g, source, Some(YOU));
    cleanup(&mut g); assert_eq!(g.state.delayed.len(), 1);
    turn(&mut g, THEM, 11, false); cleanup(&mut g); assert_eq!(g.state.delayed.len(), 1);
    turn(&mut g, YOU, 12, false); step(&mut g, Step::Upkeep); assert_eq!(g.state.triggers.len(), 1);
}

#[test]
fn an_ordinary_next_upkeep_still_allows_an_extra_upkeep_in_the_current_turn() {
    let (mut g, source) = game(); let ability = AbilityRef::delayed(g.state.obj(source).card, 0);
    cx(&mut g).delayed_trigger(DelayedWhen::Step { step: Step::Upkeep, on_turn_of: None }, source, YOU, ability, Target::None);
    step(&mut g, Step::Upkeep); assert_eq!(g.state.triggers.len(), 1); assert!(g.state.delayed.is_empty());
}

#[test]
fn a_departed_players_future_step_is_discarded_when_their_turn_would_have_begun() {
    let (mut g, source) = game_with(3); schedule(&mut g, source, Some(THEM));
    g.state.update_player(THEM, |p| p.lost = true);
    g.state.update_turn(|t| t.set_step(Step::Cleanup));
    crate::turn::next_step(&mut g);
    assert_eq!(g.state.turn.active, PlayerId(2));
    assert!(g.state.delayed.is_empty()); assert!(g.state.triggers.is_empty());
}

#[test]
fn rolling_back_and_replaying_the_step_restores_and_consumes_the_same_delayed_ability() {
    let (mut g, source) = game(); schedule(&mut g, source, None); turn(&mut g, THEM, 11, false);
    let mark = g.state.mark(); let log = g.log.len(); let before = state_hash(&g);
    step(&mut g, Step::Upkeep); let after = state_hash(&g); assert!(g.state.delayed.is_empty());
    g.state.rollback(mark); g.log.truncate(log); assert_eq!(state_hash(&g), before); assert_eq!(g.state.delayed.len(), 1);
    step(&mut g, Step::Upkeep); assert_eq!(state_hash(&g), after); assert_eq!(g.state.triggers.len(), 1);
}
