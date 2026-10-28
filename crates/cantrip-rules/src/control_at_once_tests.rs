//! Control settled at once (CR 613.1b, 611.2b), not only as the state-based actions are checked: an Aura that gave control
//! leaving, and a "for as long as" condition turning false, give the permanent back before anything reads it next, over
//! test cards.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn enchanted(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    q.state().obj(source).attached_to == Some(affected)
}

/// "You control enchanted creature."
static MIND: CardDef = CardDef {
    name: "Control Test Mind",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    statics: &[control_changing(enchanted, "")],
    ..CardDef::BLANK
};

/// "... for as long as that creature's power remains 2 or less."
fn small(q: &dyn Query, e: &ContinuousEffect) -> bool {
    q.chars(e.affected).power.is_some_and(|p| p <= 2)
}

static OLD_MAN: CardDef = CardDef {
    name: "Control Test Old Man",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(3),
    lasting: &[Lasting { text: "", holds: small }],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Control Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&MIND, &OLD_MAN, &BEAR])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

/// Your Aura gives you their Bear; as it is put into the graveyard, the Bear is theirs again at once, before the
/// state-based actions are checked.
#[test]
fn an_aura_that_gave_control_leaving_gives_the_creature_back_at_once() {
    let (mut game, table) = game();
    let bear = put(&mut game, table, &BEAR, THEM);
    let mind = put(&mut game, table, &MIND, YOU);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, YOU);
    crate::ops::move_to(&mut cx, mind, Zone::graveyard(YOU));
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

/// The Old Man takes their Bear for as long as its power is 2 or less; the moment an effect makes it 3, the Bear is
/// theirs again (CR 611.2b), not when the state-based actions are next checked.
#[test]
fn a_for_as_long_as_condition_turning_false_ends_the_control_at_once() {
    let (mut game, table) = game();
    let old_man = put(&mut game, table, &OLD_MAN, YOU);
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    let lasting = Duration::AsLongAs(table.id_of(&OLD_MAN), 0);
    cantrip_api::Game::add_effect(&mut cx, old_man, bear, EffectKind::Control(YOU), lasting);
    assert_eq!(cx.s().obj(bear).controller, YOU);
    cantrip_api::Game::add_effect(&mut cx, old_man, bear, EffectKind::Pump(1, 0), Duration::EndOfTurn);
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

/// "... for as long as you control an artifact."
fn you_control_an_artifact(q: &dyn Query, e: &ContinuousEffect) -> bool {
    q.state().battlefield().iter().any(|&id| q.state().obj(id).controller == e.controller && q.chars(id).is(CardType::Artifact))
}

static KEEPER: CardDef = CardDef {
    name: "Control Test Keeper",
    types: TypeSet::of(&[CardType::Enchantment]),
    lasting: &[Lasting { text: "", holds: you_control_an_artifact }],
    ..CardDef::BLANK
};

static RELIC: CardDef = CardDef { name: "Control Test Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// You hold their Bear for as long as you control an artifact. An exchange of your Relic for theirs is one instruction
/// (CR 701.12a): there is no moment between its halves in which you control no artifact, so the Bear stays yours.
#[test]
fn an_exchange_of_control_leaves_no_moment_between_its_halves() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&KEEPER, &RELIC, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let keeper = put(&mut game, table, &KEEPER, YOU);
    let yours = put(&mut game, table, &RELIC, YOU);
    let theirs = put(&mut game, table, &RELIC, THEM);
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, keeper, bear, EffectKind::Control(YOU), Duration::AsLongAs(table.id_of(&KEEPER), 0));
    assert_eq!(cx.s().obj(bear).controller, YOU);
    assert!(cantrip_api::Game::exchange_control(&mut cx, keeper, yours, theirs));
    assert_eq!((cx.s().obj(yours).controller, cx.s().obj(theirs).controller), (THEM, YOU));
    assert_eq!(cx.s().obj(bear).controller, YOU);
}

/// An instruction that panics leaves settling as it found it: a host that catches the panic goes on with the game.
#[test]
fn a_panicking_instruction_does_not_leave_settling_held() {
    let (mut game, _) = game();
    let mut cx = cx(&mut game);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::ops::as_one_instruction(&mut cx, |_| panic!("the instruction fails"));
    }));
    assert!(caught.is_err());
    assert_eq!(cx.s().settle_held.get(), 0);
}

// --- Layer 2's order (CR 613.1b, 613.8): control-changing effects that decide one another.

fn creature_an_opponent_controls(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature) && q.controller(affected) != q.controller(source)
}

/// "You control each creature an opponent controls."
static USURPER: CardDef = CardDef {
    name: "Control Test Usurper",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[control_changing(creature_an_opponent_controls, "")],
    ..CardDef::BLANK
};

fn attached_permanent(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    q.state().obj(source).attached_to == Some(affected)
}

/// "You control enchanted permanent."
static LEASH: CardDef = CardDef {
    name: "Control Test Leash",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    statics: &[control_changing(attached_permanent, "")],
    ..CardDef::BLANK
};

fn creatures(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

/// "You control all creatures."
static TYRANT: CardDef = CardDef {
    name: "Control Test Tyrant",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[control_changing(creatures, "")],
    ..CardDef::BLANK
};

static FIELD: CardDef = CardDef { name: "Control Test Field", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn layer_two_game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&USURPER, &LEASH, &TYRANT, &FIELD, &BEAR])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

/// CR 613.8a: their Leash takes your Bear; your older Usurper takes each creature an opponent controls. Applying the Leash
/// changes what the Usurper applies to (the Bear becomes an opponent's), so the Usurper depends on it and applies after it:
/// the Bear is yours. In timestamp order the Leash would win.
#[test]
fn a_control_effect_whose_reach_another_changes_waits_for_it() {
    let (mut game, table) = layer_two_game();
    put(&mut game, table, &USURPER, YOU);
    let bear = put(&mut game, table, &BEAR, YOU);
    let leash = put(&mut game, table, &LEASH, THEM);
    game.state.update_obj(leash, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, YOU);
}

/// CR 613.8b: two Leashes, each on the other. Each gives control to its own controller, which the other decides: a loop,
/// applied in timestamp order. The earlier Leash's controller takes the later; then the later, now that player's, gives
/// them the earlier: both go to the earlier Leash's controller.
#[test]
fn a_control_dependency_loop_applies_in_timestamp_order() {
    for (first, second) in [(YOU, THEM), (THEM, YOU)] {
        let (mut game, table) = layer_two_game();
        let a = put(&mut game, table, &LEASH, first);
        let b = put(&mut game, table, &LEASH, second);
        game.state.update_obj(a, |o| o.attached_to = Some(b));
        game.state.update_obj(b, |o| o.attached_to = Some(a));
        let mut cx = cx(&mut game);
        crate::ops::settle_control(&mut cx);
        assert_eq!((cx.s().obj(a).controller, cx.s().obj(b).controller), (first, first));
    }
}

/// CR 613.1b, 613.1d: "you control all creatures" applies in layer 2, to what layer 1 made each permanent: a land an effect
/// makes a creature in layer 4 is no creature there, and stays yours; your Bear is theirs.
#[test]
fn a_control_effect_does_not_reach_a_land_animated_in_layer_four() {
    let (mut game, table) = layer_two_game();
    put(&mut game, table, &TYRANT, THEM);
    let field = put(&mut game, table, &FIELD, YOU);
    let bear = put(&mut game, table, &BEAR, YOU);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, field, field, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
    crate::ops::settle_control(&mut cx);
    assert!(cx.chars(field).is(CardType::Creature));
    assert_eq!(cx.s().obj(field).controller, YOU);
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

static PLAIN: CardDef = CardDef { name: "Control Test Plain", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };

/// "... for as long as you control [this]": the effect's source.
fn you_control_its_source(q: &dyn Query, e: &ContinuousEffect) -> bool {
    q.state().obj(e.source).controller == e.controller
}

static HOLDER: CardDef = CardDef {
    name: "Control Test Holder",
    types: TypeSet::of(&[CardType::Enchantment]),
    lasting: &[Lasting { text: "", holds: you_control_its_source }],
    ..CardDef::BLANK
};

fn more_game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LEASH, &TYRANT, &PLAIN, &HOLDER, &BEAR])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

/// CR 611.2b, 613.1b: their plain enchantment on your Bear is a copy of a Leash until end of turn, and so gives them the
/// Bear. At cleanup the copy effect ends, and the Bear is yours again at once - before the untap step reads who controls it.
#[test]
fn a_copy_effect_ending_at_cleanup_gives_back_what_its_copy_controlled() {
    let (mut game, table) = more_game();
    let bear = put(&mut game, table, &BEAR, YOU);
    let plain = put(&mut game, table, &PLAIN, THEM);
    game.state.update_obj(plain, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    cantrip_api::Game::become_copy(&mut cx, plain, plain, Copiable::of(table.id_of(&LEASH)), Duration::EndOfTurn);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, THEM);
    crate::turn::cleanup(&mut cx).expect("a cleanup with nothing to ask");
    assert_eq!(cx.s().obj(bear).controller, YOU);
}

/// CR 702.26, 613.1b: their Leash on your Bear phases out (the Bear is yours), and phases in during their untap step: the
/// Bear is theirs again as it does.
#[test]
fn an_aura_phasing_in_gives_control_of_its_creature() {
    let (mut game, table) = more_game();
    let bear = put(&mut game, table, &BEAR, YOU);
    let leash = put(&mut game, table, &LEASH, THEM);
    game.state.update_obj(leash, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, THEM);
    crate::phasing::phase_out(&mut cx, &[leash], None, false);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, YOU);
    crate::phasing::untap_step(&mut cx, THEM);
    assert_eq!(cx.s().obj(leash).zone, Zone::BATTLEFIELD);
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

/// CR 611.2b, 613.1b: you control their Bear for as long as you control your Holder. Their Leash takes the Holder; that
/// ends the effect, and the Bear is theirs again - settled at once, the one change after the other.
#[test]
fn a_for_as_long_as_you_control_effect_ends_when_control_of_its_source_changes() {
    let (mut game, table) = more_game();
    let holder = put(&mut game, table, &HOLDER, YOU);
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, holder, bear, EffectKind::Control(YOU), Duration::AsLongAs(table.id_of(&HOLDER), 0));
    assert_eq!(cx.s().obj(bear).controller, YOU);
    let leash = put(cx.g, table, &LEASH, THEM);
    cx.st().update_obj(leash, |o| o.attached_to = Some(holder));
    crate::ops::settle_at_once(&mut cx);
    assert_eq!(cx.s().obj(holder).controller, THEM);
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

/// CR 702.26b, 702.26d: a phased-out permanent is treated as though it doesn't exist; "you control all creatures" does not
/// reach your phased-out Bear.
#[test]
fn a_phased_out_permanent_is_untouched_by_a_control_changing_ability() {
    let (mut game, table) = more_game();
    put(&mut game, table, &TYRANT, THEM);
    let bear = put(&mut game, table, &BEAR, YOU);
    let mut cx = cx(&mut game);
    crate::phasing::phase_out(&mut cx, &[bear], None, false);
    let order = crate::layers::control_order(cx.cards(), cx.s(), &cx.g.log);
    assert!(order.contains(&(bear, YOU, false)), "{order:?}");
}

// --- "For as long as [its source] remains on the battlefield" (CR 611.2b): over the moment the source is gone.

static FLYER: CardDef = CardDef {
    name: "Control Test Flyer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

fn source_game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PLAIN, &BEAR, &FLYER])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

/// You control their Bear for as long as your Plain remains on the battlefield. Sacrificing the Plain gives the Bear back
/// as it is sacrificed - in the middle of a cost being paid as much as anywhere - not when the state-based actions are next
/// checked.
#[test]
fn control_for_as_long_as_its_source_remains_ends_as_the_source_leaves() {
    let (mut game, table) = source_game();
    let plain = put(&mut game, table, &PLAIN, YOU);
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, plain, bear, EffectKind::Control(YOU), Duration::WhileSourceOnBattlefield);
    assert_eq!(cx.s().obj(bear).controller, YOU);
    crate::ops::sacrifice(&mut cx, YOU, &[plain]);
    assert_eq!(cx.s().obj(bear).controller, THEM);
    assert!(cx.s().effects.is_empty());
}

/// Your Flyer gets +2/+0 and loses all abilities for as long as your Plain remains on the battlefield; the moment the
/// Plain is put into the graveyard it is a 1/1 flyer again (layers 6 and 7c).
#[test]
fn characteristics_for_as_long_as_their_source_remains_end_as_the_source_leaves() {
    let (mut game, table) = source_game();
    let plain = put(&mut game, table, &PLAIN, YOU);
    let flyer = put(&mut game, table, &FLYER, YOU);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, plain, flyer, EffectKind::Pump(2, 0), Duration::WhileSourceOnBattlefield);
    cantrip_api::Game::add_effect(&mut cx, plain, flyer, EffectKind::LoseAllAbilities, Duration::WhileSourceOnBattlefield);
    assert_eq!(cx.chars(flyer).power, Some(3));
    assert!(!cx.chars(flyer).has(Keyword::Flying));
    crate::ops::move_to(&mut cx, plain, Zone::graveyard(YOU));
    assert_eq!(cx.chars(flyer).power, Some(1));
    assert!(cx.chars(flyer).has(Keyword::Flying));
}

/// CR 702.26f: the duration tracks the Plain, which phasing out it can no longer see: the Bear is theirs again as the
/// Plain phases out, and stays theirs when it phases back in.
#[test]
fn control_for_as_long_as_its_source_remains_ends_as_the_source_phases_out() {
    let (mut game, table) = source_game();
    let plain = put(&mut game, table, &PLAIN, YOU);
    let bear = put(&mut game, table, &BEAR, THEM);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_effect(&mut cx, plain, bear, EffectKind::Control(YOU), Duration::WhileSourceOnBattlefield);
    assert_eq!(cx.s().obj(bear).controller, YOU);
    crate::phasing::phase_out(&mut cx, &[plain], None, false);
    assert_eq!(cx.s().obj(bear).controller, THEM);
    crate::phasing::untap_step(&mut cx, YOU);
    assert_eq!(cx.s().obj(plain).zone, Zone::BATTLEFIELD);
    crate::ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(bear).controller, THEM);
}

/// A permanent coming under a player's control is a record in the history (CR 613.1b, 702.30a): once as the Aura gives it
/// to you, once as it goes back to its owner; a permanent whose controller stays the same leaves none.
#[test]
fn a_change_of_control_is_recorded_each_time_it_happens() {
    let (mut game, table) = game();
    let bear = put(&mut game, table, &BEAR, THEM);
    let mind = put(&mut game, table, &MIND, YOU);
    game.state.update_obj(mind, |o| o.attached_to = Some(bear));
    let mut cx = cx(&mut game);
    let changes = |cx: &Cx| -> Vec<PlayerId> {
        cx.g.log.iter().filter_map(|r| match r.event { Event::ControlChanged { permanent, controller } if permanent == bear => Some(controller), _ => None }).collect()
    };
    crate::ops::settle_control(&mut cx);
    crate::ops::settle_control(&mut cx);
    assert_eq!(changes(&cx), vec![YOU], "settling again changes nothing and records nothing");
    crate::ops::move_to(&mut cx, mind, Zone::graveyard(YOU));
    assert_eq!(changes(&cx), vec![YOU, THEM]);
}
