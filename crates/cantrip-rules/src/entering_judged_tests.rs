//! A permanent about to enter is judged as it would exist on the battlefield (CR 614.12), over test cards: with what leaves
//! the battlefield in the same move gone (CR 101.4), its own static abilities among those that could take its abilities,
//! entering tapped as it will, again after a replacement changes how it enters, and with the replacements of its own read
//! off what it enters as.

use crate::ops::{self, Moving};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn noncreature_artifact(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact) && !c.is(CardType::Creature)
}

fn tapped_artifact(q: &dyn LayerQuery, _: ObjectId, id: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact) && q.state().obj(id).tapped
}

fn your_artifact(q: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact) && q.state().obj(id).controller == q.state().obj(source).controller
}

fn add_creature(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.types = c.types.with(CardType::Creature);
}

fn two_counters(_: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += 2;
    Ok(())
}

/// "Creatures you control can't have -1/-1 counters put on them."
fn no_minus_counters(q: &dyn Query, asker: &Asker, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
    kind == CounterKind::MinusOne && controller == asker.controller && q.chars(object).is(CardType::Creature)
}

static BAN: CardDef =
    CardDef { name: "Judged Ban", types: TypeSet::of(&[CardType::Enchantment]), counter_bans: &[no_minus_counters], ..CardDef::BLANK };
/// "Each noncreature artifact is an artifact creature."
static ANIMATOR: CardDef = CardDef {
    name: "Judged Animator",
    types: TypeSet::of(&[CardType::Artifact]),
    statics: &[static_ability(Layer::Type, noncreature_artifact, add_creature, "")],
    ..CardDef::BLANK
};
/// "Tapped artifacts are creatures."
static TAPPED_ANIMATOR: CardDef = CardDef {
    name: "Judged Tapped Animator",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, tapped_artifact, add_creature, "")],
    ..CardDef::BLANK
};
/// "Artifacts you control are creatures."
static YOUR_ANIMATOR: CardDef = CardDef {
    name: "Judged Your Animator",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, your_artifact, add_creature, "")],
    ..CardDef::BLANK
};
static RELIC: CardDef = CardDef { name: "Judged Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
/// A creature that enters with two +1/+1 counters, and whose own "creatures lose all abilities" takes that from it.
static HUMBLE: CardDef = CardDef {
    name: "Judged Humble",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    enters: Some(two_counters),
    statics: &[loses_all_abilities(creature, "")],
    ..CardDef::BLANK
};
static GROWN: CardDef = CardDef { name: "Judged Grown", statics: &[], ..HUMBLE };
/// The Aura face of a creature with bestow, which has its "enters with two +1/+1 counters" as well (CR 702.103b).
static BESTOWED: CardDef = CardDef {
    name: "Judged Bestowed",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    power: None,
    toughness: None,
    ..GROWN
};
static BESTOWER: CardDef = CardDef { name: "Judged Bestower", bestow: Some(&BESTOWED), ..GROWN };

/// "This creature enters tapped", a replacement of its own that functions from every zone (CR 113.6b, 614.1d).
fn this_enters(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.object == asker.source && z.to.is(ZoneKind::Battlefield))
}

fn enters_tapped(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.tapped = true;
    }
}

static SLEEPER: CardDef = CardDef {
    name: "Judged Sleeper",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    replacements: &[replacement(ReplacementKind::ZoneChange, this_enters, enters_tapped, "").own_everywhere()],
    ..CardDef::BLANK
};
static PLAIN: CardDef = CardDef { name: "Judged Plain", replacements: &[], ..SLEEPER };

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &BAN,
        &ANIMATOR,
        &TAPPED_ANIMATOR,
        &YOUR_ANIMATOR,
        &RELIC,
        &HUMBLE,
        &GROWN,
        &SLEEPER,
        &PLAIN,
        &BESTOWED,
        &BESTOWER,
    ])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), controller, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn entering(id: ObjectId, controller: PlayerId, tapped: bool) -> Entering {
    Entering {
        object: id,
        controller,
        tapped,
        counters: [0; CounterKind::COUNT],
        copy: None,
        prepared: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_object: None,
        attach: None,
        attach_player: None,
        x: 0,
        transformed: false,
        protector: None,
        instead: None,
        becomes: None,
        own: None,
    }
}

/// Goblin Welder's move (CR 101.4): the Animator is sacrificed as the Relic returns "with a -1/-1 counter", so the Relic is
/// judged with the Animator gone - no creature, which the Ban does not reach - and gets its counter. With the Animator
/// staying, it would be a creature there and gets none.
#[test]
fn a_permanent_leaving_in_the_same_move_is_gone_as_the_entering_one_is_judged() {
    for leaves in [true, false] {
        let (mut game, table) = game();
        put(&mut game, table, &BAN, Zone::BATTLEFIELD, P0);
        let animator = put(&mut game, table, &ANIMATOR, Zone::BATTLEFIELD, P0);
        let relic = put(&mut game, table, &RELIC, Zone::graveyard(P0), P0);
        let mut cx = cx(&mut game);
        let moves = if leaves { vec![Moving::to(animator, Zone::graveyard(P0))] } else { Vec::new() };
        let new = ops::put_onto_battlefield_along(&mut cx, moves, &[(relic, P0, false)], ZoneReason::Other, &[(CounterKind::MinusOne, 1)])
            .expect("nothing is asked");
        let new = *new.last().expect("the Relic entered");
        assert_eq!(cx.s().obj(new).counters(CounterKind::MinusOne), if leaves { 1 } else { 0 });
        assert!(cx.s().leaving_together.borrow().is_empty());
    }
}

/// Its own "creatures lose all abilities" would take its "enters with two +1/+1 counters" there (CR 614.12), with nothing
/// else on the battlefield to say any ability could be lost.
#[test]
fn its_own_static_ability_can_take_its_enters_with_abilities() {
    let (mut game, table) = game();
    let humble = put(&mut game, table, &HUMBLE, Zone::hand(P0), P0);
    let grown = put(&mut game, table, &GROWN, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    // Whether its own "enters with" is left to apply as it enters (`Entering::own`).
    let mut judged = |id| {
        let mut e = entering(id, P0, false);
        ops::prepare_entry(&mut cx, &mut e).expect("nothing is asked");
        e.own.is_some()
    };
    assert!(judged(grown));
    assert!(!judged(humble));
}

/// It is judged tapped if it enters tapped: "tapped artifacts are creatures" makes it a creature there.
#[test]
fn it_is_judged_tapped_as_it_enters_tapped() {
    let (mut game, table) = game();
    put(&mut game, table, &TAPPED_ANIMATOR, Zone::BATTLEFIELD, P0);
    let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    assert!(ops::as_it_would_exist(&mut cx, &entering(relic, P0, true)).chars.is(CardType::Creature));
    assert!(!ops::as_it_would_exist(&mut cx, &entering(relic, P0, false)).chars.is(CardType::Creature));
}

/// A replacement that has it enter under another player's control has the rest judge it under that player: P1's "artifacts
/// you control are creatures" makes it a creature only then. One that sends it elsewhere leaves no permanent to judge.
#[test]
fn it_is_judged_again_after_a_replacement_changes_how_it_enters() {
    let (mut game, table) = game();
    put(&mut game, table, &YOUR_ANIMATOR, Zone::BATTLEFIELD, P1);
    let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    let would = ops::as_it_would_exist(&mut cx, &entering(relic, P0, false));
    assert!(!would.chars.is(CardType::Creature));
    cx.entering = Some(would);
    let change = |controller, to| ZoneChange {
        object: relic,
        from: Zone::hand(P0),
        to,
        reason: ZoneReason::Other,
        controller,
        tapped: false,
        counters: [0; CounterKind::COUNT],
        counters_by: controller,
        reveal: false,
        shuffle: false,
        is_card: true,
        madness: false,
        pays_characteristic: false,
        exiled_with: None,
        flashback: false,
        exile_instead: ExileInstead::Never,
        adventure: false,
        omen: false,
        rebound: false,
        on_adventure: false,
        rebounding: false,
        destroy: false,
        regenerated: false,
        shielded: false,
    };
    ops::rejudge_entering(&mut cx, &change(P1, Zone::BATTLEFIELD));
    assert!(cx.entering.is_some_and(|w| w.chars.is(CardType::Creature) && w.entering.controller == P1));
    assert!(cx.chars(relic).is(CardType::Creature));
    ops::rejudge_entering(&mut cx, &change(P1, Zone::exile(P0)));
    assert!(cx.entering.is_none());
    assert!(!cx.chars(relic).is(CardType::Creature));
}

/// A card entering as a copy of the Sleeper has the Sleeper's "this enters tapped", read off what it enters as (CR 614.12,
/// 707.9a), not off the card it is where it is.
#[test]
fn its_own_replacements_are_read_off_what_it_enters_as() {
    let (mut game, table) = game();
    let plain = put(&mut game, table, &PLAIN, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    let mut e = entering(plain, P0, false);
    e.copy = Some(Copiable::of(table.id_of(&SLEEPER)));
    let moves = [Moving { entering: Some(e), ..Moving::to(plain, Zone::BATTLEFIELD) }];
    let new = ops::move_together(&mut cx, &moves, ZoneReason::Other)[0];
    assert_eq!(cx.chars(new).name, "Judged Sleeper");
    assert!(cx.s().obj(new).tapped);
}

/// "If a permanent would enter the battlefield under an opponent's control, it enters under your control instead."
fn under_an_opponent(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield) && z.controller != asker.controller)
}

fn under_you(_: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.controller = asker.controller;
    }
}

static USURPER: CardDef = CardDef {
    name: "Judged Usurper",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, under_an_opponent, under_you, "")],
    ..CardDef::BLANK
};

/// "Permanents enter tapped."
fn entering_permanent(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield))
}

static FROST: CardDef = CardDef {
    name: "Judged Frost",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, entering_permanent, enters_tapped, "")],
    ..CardDef::BLANK
};

fn game_with_more() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &BAN,
        &YOUR_ANIMATOR,
        &TAPPED_ANIMATOR,
        &RELIC,
        &USURPER,
        &FROST,
    ])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

/// P1's Usurper has P0's Relic enter under P1's control (CR 614.1c), and it does: the counters it enters with are then
/// judged under P1 (CR 614.12, 122.6), where P1's "artifacts you control are creatures" makes it a creature that P1's Ban
/// keeps the -1/-1 counter off.
#[test]
fn a_replacement_that_changes_who_it_enters_under_is_applied_and_judged() {
    let (mut game, table) = game_with_more();
    put(&mut game, table, &USURPER, Zone::BATTLEFIELD, P1);
    put(&mut game, table, &YOUR_ANIMATOR, Zone::BATTLEFIELD, P1);
    put(&mut game, table, &BAN, Zone::BATTLEFIELD, P1);
    let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    let new = ops::put_onto_battlefield_with(&mut cx, &[(relic, P0, false)], ZoneReason::Other, &[(CounterKind::MinusOne, 1)])
        .expect("nothing is asked")[0];
    assert_eq!(cx.s().obj(new).controller, P1);
    assert!(cx.chars(new).is(CardType::Creature));
    assert_eq!(cx.s().obj(new).counters(CounterKind::MinusOne), 0);
}

/// The Frost has the Relic enter tapped; its counters are judged on it tapped, a creature under "tapped artifacts are
/// creatures", which the Ban keeps the -1/-1 counter off.
#[test]
fn its_counters_are_judged_as_the_replacements_left_it_entering() {
    let (mut game, table) = game_with_more();
    put(&mut game, table, &FROST, Zone::BATTLEFIELD, P0);
    put(&mut game, table, &TAPPED_ANIMATOR, Zone::BATTLEFIELD, P0);
    put(&mut game, table, &BAN, Zone::BATTLEFIELD, P0);
    let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    let new = ops::put_onto_battlefield_with(&mut cx, &[(relic, P0, false)], ZoneReason::Other, &[(CounterKind::MinusOne, 1)])
        .expect("nothing is asked")[0];
    assert!(cx.s().obj(new).tapped);
    assert_eq!(cx.s().obj(new).counters(CounterKind::MinusOne), 0);
}

static LEFT_DOOR: CardDef = CardDef {
    name: "Judged Left Door",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Room]),
    ..CardDef::BLANK
};
static RIGHT_DOOR: CardDef = CardDef { name: "Judged Right Door", ..LEFT_DOOR };
static ROOM: CardDef = CardDef { name: "Judged Left Door // Judged Right Door", split: Some((&LEFT_DOOR, &RIGHT_DOOR)), ..LEFT_DOOR };

fn protected_by_its_owner(q: &dyn LayerQuery, _: ObjectId, id: ObjectId, c: &Chars) -> bool {
    let o = q.state().obj(id);
    c.is(CardType::Artifact) && o.marks.protector == Some(o.owner)
}

/// "Artifacts their owners protect are creatures": what reads a battle's protector, here on an artifact.
static GUARD: CardDef = CardDef {
    name: "Judged Guard",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, protected_by_its_owner, add_creature, "")],
    ..CardDef::BLANK
};

/// The trial object is a Room spell cast for its left door, entering with that door unlocked (CR 709.5d), so it has that
/// door's name alone; and one entering with a protector has it (CR 310.9a).
#[test]
fn it_is_judged_with_its_cast_door_and_its_protector() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LEFT_DOOR, &RIGHT_DOOR, &ROOM, &GUARD, &RELIC])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let room = put(&mut game, table, &ROOM, Zone::STACK, P0);
    game.state.update_obj(room, |o| o.cast_way = WAY_SPLIT_LEFT);
    put(&mut game, table, &GUARD, Zone::BATTLEFIELD, P0);
    let relic = put(&mut game, table, &RELIC, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    assert_eq!(ops::as_it_would_exist(&mut cx, &entering(room, P0, false)).chars.name, "Judged Left Door");
    let mut protected = entering(relic, P0, false);
    protected.protector = Some(P0);
    assert!(ops::as_it_would_exist(&mut cx, &protected).chars.is(CardType::Creature));
    assert!(!ops::as_it_would_exist(&mut cx, &entering(relic, P0, false)).chars.is(CardType::Creature));
}

/// A bestowed Aura enters as its card's Aura face (CR 702.103b), whose copy values are over its card: its "enters with two
/// +1/+1 counters" is the Aura face's, applied once - never its creature face's as well.
#[test]
fn a_bestowed_aura_applies_its_enters_with_once() {
    let (mut game, table) = game();
    let bestower = put(&mut game, table, &BESTOWER, Zone::hand(P0), P0);
    let mut cx = cx(&mut game);
    let mut e = entering(bestower, P0, false);
    e.copy = Some(Copiable::of(table.id_of(&BESTOWED)));
    ops::prepare_entry(&mut cx, &mut e).expect("nothing is asked");
    assert_eq!(e.own, Some(table.id_of(&BESTOWED)), "the Aura face's alone");
    ops::apply_own_entering(&mut cx, &mut e, P0, false);
    assert_eq!(e.counters[CounterKind::PlusOne as usize], 2);
}
