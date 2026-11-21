//! A trigger on a player losing control of an object, or an opponent gaining control of one from that player, looks back in
//! time (CR 603.10d): it is judged on the game just before the change, so a permanent that is itself taken triggers under the
//! controller it had, and one that changes hands with another does too. A trigger on a gain of control that says no more is
//! judged after the change. Test-only cards: a watcher "whenever an opponent gains control of a permanent from you", the same
//! for "whenever an opponent gains control of a permanent" and a plain creature.

use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn from_you(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::ControlChanged { permanent, from, controller } if from == me.controller && controller != me.controller => Fired::about(Target::Object(permanent)),
        _ => None,
    }
}

fn to_an_opponent(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::ControlChanged { permanent, controller, .. } if controller != me.controller => Fired::about(Target::Object(permanent)),
        _ => None,
    }
}

static LOSS_WATCHER: CardDef = CardDef {
    name: "Control Look Back Loss Watcher",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when_asked(EventMask::CONTROL_CHANGED, from_you), nothing, "Whenever an opponent gains control of a permanent from you, nothing happens.")],
    ..CardDef::BLANK
};

static GAIN_WATCHER: CardDef = CardDef {
    name: "Control Look Back Gain Watcher",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when_asked(EventMask::CONTROL_CHANGED, to_an_opponent), nothing, "Whenever an opponent gains control of a permanent, nothing happens.")],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef { name: "Control Look Back Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LOSS_WATCHER, &GAIN_WATCHER, &BEAR])));
    let mut g = Game::new(table, &Setup { decks: vec![vec![table.id_of(&BEAR); 20]; 2], shuffle: false, ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.began = true;
        t.set_step(Step::PrecombatMain);
    });
    g.state.update_player(YOU, |p| p.turn_began = 3);
    (g, table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), controller, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// Their one instruction takes each of `affected` at once (CR 613.1b); the triggers that waited to go on the stack, as
/// (source, controller).
fn taken(game: &mut Game, thief: ObjectId, affected: &[ObjectId]) -> Vec<(ObjectId, PlayerId)> {
    let mut cx = cx(game);
    crate::ops::as_one_instruction(&mut cx, |cx| {
        for &id in affected {
            Rules::add_effect(cx, thief, id, EffectKind::Control(THEM), Duration::EndOfTurn);
        }
    });
    game.state.triggers.iter().map(|t| (t.source, t.controller)).collect()
}

/// The watcher itself is taken: it triggers, under you - the controller it had just before the change (CR 603.10d) - though it is
/// theirs by the time the trigger is looked for.
#[test]
fn a_watcher_that_is_taken_triggers_under_its_former_controller() {
    let (mut game, table) = game();
    let watcher = put(&mut game, table, &LOSS_WATCHER, YOU);
    let thief = put(&mut game, table, &BEAR, THEM);
    assert_eq!(taken(&mut game, thief, &[watcher]), vec![(watcher, YOU)]);
    assert_eq!(game.state.obj(watcher).controller, THEM);
}

/// One of your other permanents is taken: the watcher, still yours, triggers once for it.
#[test]
fn a_watcher_triggers_for_another_permanent_taken() {
    let (mut game, table) = game();
    let watcher = put(&mut game, table, &LOSS_WATCHER, YOU);
    let bear = put(&mut game, table, &BEAR, YOU);
    let thief = put(&mut game, table, &BEAR, THEM);
    assert_eq!(taken(&mut game, thief, &[bear]), vec![(watcher, YOU)]);
}

/// Taken together with another of your permanents, the watcher triggers for both, under you: what it sees is the game as it was before
/// either change (CR 603.10d, 603.2c).
#[test]
fn a_watcher_taken_with_another_permanent_triggers_for_both() {
    let (mut game, table) = game();
    let watcher = put(&mut game, table, &LOSS_WATCHER, YOU);
    let bear = put(&mut game, table, &BEAR, YOU);
    let thief = put(&mut game, table, &BEAR, THEM);
    let triggers = taken(&mut game, thief, &[watcher, bear]);
    assert_eq!(triggers, vec![(watcher, YOU), (watcher, YOU)]);
}

/// A permanent an opponent already controls changing hands to another opponent is not "from you": the change is from them.
#[test]
fn a_permanent_taken_from_another_player_does_not_trigger() {
    let (mut game, table) = game();
    put(&mut game, table, &LOSS_WATCHER, YOU);
    let bear = put(&mut game, table, &BEAR, THEM);
    let thief = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    Rules::add_effect(&mut cx, thief, bear, EffectKind::Control(YOU), Duration::EndOfTurn);
    assert!(game.state.triggers.is_empty());
}

/// A trigger on a gain of control that does not say "from you" is not one that looks back: the watcher taken by them is theirs when
/// the change is judged, and an opponent gaining control of it is not its controller's opponent gaining anything.
#[test]
fn a_trigger_on_any_gain_of_control_is_judged_after_the_change() {
    let (mut game, table) = game();
    let watcher = put(&mut game, table, &GAIN_WATCHER, YOU);
    let thief = put(&mut game, table, &BEAR, THEM);
    assert_eq!(taken(&mut game, thief, &[watcher]), Vec::new());
}

/// An effect from a spell, not a permanent (Act of Treason), takes the watcher, the only permanent on the battlefield: the change is
/// still one a look-back sees, though nothing else on the board stays as it was.
#[test]
fn a_watcher_taken_by_a_spell_with_nothing_else_on_the_battlefield_triggers_under_its_former_controller() {
    let (mut game, table) = game();
    let watcher = put(&mut game, table, &LOSS_WATCHER, YOU);
    let spell = game.state.push_obj(Obj::new(table.id_of(&BEAR), THEM, Zone::hand(THEM)));
    assert_eq!(taken(&mut game, spell, &[watcher]), vec![(watcher, YOU)]);
}
