//! "You may play that card for as long as you control [this permanent]" (`Expiry::WhileControls`, CR 611.2b): Gwen Stacy's
//! and Lightning, Security Sergeant's permission, over test cards, since neither printed card is built yet.

use crate::actions::{permissions_for, priority_options};
use crate::layers::live;
use crate::{Cx, Game, Setup, ops, phasing};
use cantrip_api::*;

static SPIDER: CardDef = CardDef {
    name: "Permission Test Spider",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
/// The back face it transforms into, without the text that gave the permission (CR 712.18).
static GHOST: CardDef = CardDef {
    name: "Permission Test Ghost",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};
static DFC: CardDef = CardDef { back: Some(&GHOST), ..SPIDER };
static LAND: CardDef = CardDef { name: "Permission Test Land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SPELL: CardDef = CardDef { name: "Permission Test Spell", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Your precombat main phase, with your double-faced creature on the battlefield and a land and a sorcery of yours in exile.
fn game() -> (Game, ObjectId, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&DFC, &GHOST, &LAND, &SPELL])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let spider = put(&mut game, &DFC, Zone::BATTLEFIELD);
    let land = put(&mut game, &LAND, Zone::exile(YOU));
    let spell = put(&mut game, &SPELL, Zone::exile(YOU));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, spider, land, spell)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn grant(source: ObjectId) -> PlayGrant {
    PlayGrant { lands: true, expiry: Expiry::WhileControls(source), ..PlayGrant::CAST }
}

/// "Exile the top card ... You may play that card for as long as you control this creature", as it resolves.
fn allow(cx: &mut Cx, source: ObjectId, cards: &[ObjectId]) {
    for &card in cards {
        cantrip_api::Game::allow_play(cx, card, YOU, grant(source));
    }
}

fn land_playable(cx: &mut Cx, land: ObjectId) -> bool {
    priority_options(cx, YOU).iter().any(|o| o.kind == ActionKind::PlayLand && o.source == Some(land))
}

#[test]
fn a_land_is_played_and_a_spell_cast_from_exile_while_you_control_it_and_transforming_it_ends_nothing() {
    let (mut game, spider, land, spell) = game();
    let mut cx = cx(&mut game);
    allow(&mut cx, spider, &[land, spell]);
    // "Play": the land is a land play from exile (CR 305.1), the sorcery may be cast (CR 601.3).
    assert!(land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_some());
    // The opponent holds no permission.
    assert!(permissions_for(&cx, spell, THEM).next().is_none());
    // CR 712.18: it transforms into a face without the text, and is the same object, so the permission goes on.
    ops::turn_over(&mut cx, spider).unwrap();
    assert!(cx.s().obj(spider).transformed && live(cx.s(), spider));
    crate::settle::state_based_actions_alone(&mut cx).unwrap();
    assert!(land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_some());
    assert_eq!(cx.s().permissions.len(), 2);
}

#[test]
fn it_ends_when_the_permanent_leaves_and_does_not_come_back_with_it() {
    let (mut game, spider, land, spell) = game();
    let mut cx = cx(&mut game);
    allow(&mut cx, spider, &[land, spell]);
    ops::move_to(&mut cx, spider, Zone::graveyard(YOU));
    // Over at once, before the state-based check drops it (CR 611.2b).
    assert!(!land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_none());
    crate::settle::state_based_actions_alone(&mut cx).unwrap();
    assert!(cx.s().permissions.is_empty());
    // Returned, it is a new object (CR 400.7): the permission is not the new one's.
    let dead = cx.s().obj(spider).became.unwrap();
    let back = ops::move_to(&mut cx, dead, Zone::BATTLEFIELD);
    assert!(live(cx.s(), back) && cx.s().obj(back).controller == YOU);
    assert!(!land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_none());
}

#[test]
fn it_ends_when_control_changes_and_stays_ended_once_control_comes_back() {
    let (mut game, spider, land, spell) = game();
    let mut cx = cx(&mut game);
    allow(&mut cx, spider, &[land, spell]);
    // An opponent gains control of it until end of turn (CR 613.1b).
    cantrip_api::Game::add_effect(&mut cx, spider, spider, EffectKind::Control(THEM), Duration::EndOfTurn);
    assert_eq!(cx.s().obj(spider).controller, THEM);
    assert!(cx.s().permissions.is_empty());
    assert!(!land_playable(&mut cx, land));
    // The effect ends and the permanent comes back to you: the permission does not begin again (CR 611.2b).
    cx.st().effects_retain(|e| !matches!(e.kind, EffectKind::Control(_)));
    ops::settle_control(&mut cx);
    assert_eq!(cx.s().obj(spider).controller, YOU);
    assert!(!land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_none());
}

#[test]
fn it_does_nothing_if_you_do_not_control_the_permanent_as_it_would_begin() {
    let (mut game, spider, land, spell) = game();
    let mut cx = cx(&mut game);
    // Stolen in response to the enters trigger (CR 611.2b: the duration never starts).
    cantrip_api::Game::add_effect(&mut cx, spider, spider, EffectKind::Control(THEM), Duration::EndOfTurn);
    allow(&mut cx, spider, &[land, spell]);
    assert!(cx.s().permissions.is_empty());
    // Nor once it has left (CR 400.7).
    let (mut game, spider, land, _) = self::game();
    let mut cx = self::cx(&mut game);
    ops::move_to(&mut cx, spider, Zone::graveyard(YOU));
    allow(&mut cx, spider, &[land]);
    assert!(cx.s().permissions.is_empty());
}

#[test]
fn it_ends_when_the_permanent_phases_out_and_stays_ended_once_it_phases_in() {
    let (mut game, spider, land, spell) = game();
    let mut cx = cx(&mut game);
    allow(&mut cx, spider, &[land, spell]);
    // CR 702.26f: a "for as long as" duration that tracks it ends, as it can no longer see it.
    phasing::phase_out(&mut cx, &[spider], None, false);
    assert!(cx.s().permissions.is_empty());
    phasing::untap_step(&mut cx, YOU);
    assert!(cx.s().obj(spider).zone.is(ZoneKind::Battlefield) && live(cx.s(), spider));
    assert!(!land_playable(&mut cx, land));
    assert!(permissions_for(&cx, spell, YOU).next().is_none());
}
