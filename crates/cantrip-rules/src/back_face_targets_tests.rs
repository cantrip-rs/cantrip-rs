//! A modal double-faced card's back face judged as that face (CR 601.3e, 712.8a), over test cards: it is cast only if its
//! targets could be announced (CR 601.2c), from a hand with priority and as an effect casts it, and a flash permission reaches
//! it alone when it is a sorcery behind a creature front. No printed castable back face in the engine has a target or is a
//! noncreature behind a creature (Jennifer Walters', Gwen Stacy's, King T'Challa's and Norman Osborn's are creatures).

use crate::actions::{cast_ways_during_resolution, priority_options, stack_item};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn artifact(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Artifact))
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// The back face, a sorcery of its own cost: "Destroy target artifact" (what it does does not matter here).
static BACK: CardDef = CardDef {
    name: "Back Face Test Shatter",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[target(artifact)], nothing),
    ..CardDef::BLANK
};
/// The front face, a creature: a modal double-faced card whose faces may each be cast (CR 712.8b).
static FRONT: CardDef = CardDef {
    name: "Back Face Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    back: Some(&BACK),
    ..CardDef::BLANK
};
static TRINKET: CardDef = CardDef { name: "Back Face Test Trinket", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static CASTER: CardDef = CardDef { name: "Back Face Test Caster", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

/// Your precombat main phase with the card in your hand, and an artifact on the battlefield if `artifact`.
fn game(artifact: bool) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FRONT, &BACK, &TRINKET, &CASTER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let card = put(&mut game, &FRONT, Zone::hand(YOU));
    if artifact {
        put(&mut game, &TRINKET, Zone::BATTLEFIELD);
    }
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, card)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The ways the card is offered to be cast with priority: 0 the front face, `WAY_BACK_FACE` the back.
fn offered(artifact: bool) -> Vec<u8> {
    let (mut game, card) = game(artifact);
    let mut ways: Vec<u8> =
        priority_options(&mut cx(&mut game), YOU).iter().filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).map(|o| o.index).collect();
    ways.sort_unstable();
    ways
}

/// The ways an effect resolving could cast it.
fn during_resolution(artifact: bool) -> Vec<u8> {
    let (mut game, card) = game(artifact);
    let caster = game.state.push_obj(Obj::new(game.cards.find("Back Face Test Caster").unwrap(), YOU, Zone::STACK));
    game.state.zone_push(Zone::STACK, caster);
    let item = stack_item(StackKind::Spell, caster, AbilityRef { card: game.cards.find("Back Face Test Caster").unwrap(), index: 0 }, YOU, &[]);
    game.state.stack_push(item);
    // The caster is resolving: its frame names it (`Cx::resolving`).
    let (serial, controller) = game.state.stack.last().map(|i| (i.serial, i.controller)).expect("the caster");
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len() - 1;
    let mut resolving = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    let mut ways: Vec<u8> = cast_ways_during_resolution(&mut resolving, card, controller).iter().map(|w| w.way).collect();
    ways.sort_unstable();
    ways
}

#[test]
fn a_back_face_with_no_legal_target_is_not_offered_from_the_hand() {
    assert_eq!(offered(false), [0], "the back face is offered with no artifact to target");
    assert_eq!(offered(true), [0, WAY_BACK_FACE], "the back face is not offered with an artifact to target");
}

#[test]
fn a_back_face_with_no_legal_target_is_not_among_the_ways_an_effect_casts() {
    assert_eq!(during_resolution(false), [0]);
    assert_eq!(during_resolution(true), [0, WAY_BACK_FACE]);
}

/// "You may cast noncreature spells as though they had flash" (Valley Floodcaller's).
fn noncreature_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && !q.chars(spell).is(CardType::Creature)
}
static FLASHER: CardDef = CardDef { name: "Back Face Test Flasher", types: TypeSet::of(&[CardType::Enchantment]), flash_rules: &[noncreature_you_cast], ..CardDef::BLANK };
/// A sorcery back face with no target, behind a creature front face.
static SORCERY_BACK: CardDef = CardDef { name: "Back Face Test Ponder", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };
static SORCERY_FRONT: CardDef = CardDef {
    name: "Back Face Test Wolf",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    back: Some(&SORCERY_BACK),
    ..CardDef::BLANK
};

/// The ways the card in your hand is offered on your opponent's turn, with a "noncreature spells as though they had flash"
/// permanent of yours or without.
fn offered_on_their_turn(flasher: bool) -> Vec<u8> {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&SORCERY_FRONT, &SORCERY_BACK, &FLASHER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let card = game.state.push_obj(Obj::new(table.id_of(&SORCERY_FRONT), YOU, Zone::hand(YOU)));
    game.state.zone_push(Zone::hand(YOU), card);
    if flasher {
        let id = game.state.push_obj(Obj::new(table.id_of(&FLASHER), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    game.state.update_turn(|t| {
        t.active = PlayerId(1);
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    let mut ways: Vec<u8> =
        priority_options(&mut cx(&mut game), YOU).iter().filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).map(|o| o.index).collect();
    ways.sort_unstable();
    ways
}

/// CR 601.3e, 712.8a: a flash permission for noncreature spells is judged on each face as it would be cast - the sorcery back
/// face may be cast on the opponent's turn, the creature front face may not (`actions::Speed::face`).
#[test]
fn a_flash_permission_for_noncreature_spells_reaches_a_sorcery_back_face_and_not_its_creature_front() {
    assert_eq!(offered_on_their_turn(false), Vec::<u8>::new(), "a face is offered on the opponent's turn with no permission");
    assert_eq!(offered_on_their_turn(true), [WAY_BACK_FACE], "the permission is not judged face by face");
}
