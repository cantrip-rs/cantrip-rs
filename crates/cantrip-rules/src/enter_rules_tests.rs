//! A static ability that keeps cards in some zones from entering the battlefield (`CardDef::enter_rules`, CR 101.2), over test
//! cards: the card is judged as it exists where it is, not as the effect putting it there would make it (Grafdigger's Cage's
//! ruling), whichever way it is put there, and a land card that can't enter can't be played as an effect resolves either
//! (CR 305.1). The scenarios show the rest with Grafdigger's Cage itself.

use crate::actions::{could_play_land_during_resolution, stack_item};
use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

/// "Creature cards in graveyards and libraries can't enter the battlefield."
fn no_creature_cards_from_graveyards_or_libraries(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    !(matches!(q.obj(card).zone.kind(), ZoneKind::Graveyard | ZoneKind::Library) && q.chars(card).is(CardType::Creature))
}

static CAGE: CardDef = CardDef {
    name: "Enter Rule Test Cage",
    types: TypeSet::of(&[CardType::Artifact]),
    enter_rules: &[no_creature_cards_from_graveyards_or_libraries],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Enter Rule Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static RELIC: CardDef = CardDef { name: "Enter Rule Test Relic", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static ARBOR: CardDef = CardDef {
    name: "Enter Rule Test Arbor",
    types: TypeSet::of(&[CardType::Land, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static WASTE: CardDef = CardDef { name: "Enter Rule Test Waste", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static CASTER: CardDef = CardDef { name: "Enter Rule Test Caster", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

/// "It's an enchantment" (Enduring Curiosity's return).
static AN_ENCHANTMENT: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Enchantment])), ..EntersAs::NONE };
/// "It's an artifact creature."
static AN_ARTIFACT_CREATURE: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Artifact, CardType::Creature])), ..EntersAs::NONE };

/// Your precombat main phase with the Cage on the battlefield.
fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&CAGE, &BEAR, &RELIC, &ARBOR, &WASTE, &CASTER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    put(&mut game, table, &CAGE, Zone::BATTLEFIELD);
    (game, table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// A creature card in a graveyard put onto the battlefield as an enchantment can't enter: it is a creature card where it is.
/// A noncreature artifact card put there as an artifact creature can: it is no creature card where it is.
#[test]
fn a_card_put_onto_the_battlefield_as_something_else_is_judged_as_the_card_it_is() {
    let (mut game, table) = game();
    let bear = put(&mut game, table, &BEAR, Zone::graveyard(YOU));
    let relic = put(&mut game, table, &RELIC, Zone::graveyard(YOU));
    let mut cx = cx(&mut game);
    let becomes = |what| Becomes { what, card: table.id_of(&CAGE), words: Words::NONE };
    let bear_entered = ops::put_onto_battlefield_becoming(&mut cx, bear, YOU, becomes(&AN_ENCHANTMENT)).expect("nothing is asked");
    assert_eq!(bear_entered, None);
    assert!(cx.s().zone(Zone::graveyard(YOU)).contains(&bear));
    let relic_entered = ops::put_onto_battlefield_becoming(&mut cx, relic, YOU, becomes(&AN_ARTIFACT_CREATURE)).expect("nothing is asked");
    let relic_entered = relic_entered.expect("the Relic entered");
    assert!(cx.s().battlefield().contains(&relic_entered));
    assert!(cx.chars(relic_entered).is(CardType::Creature));
}

/// Put onto the battlefield one at a time from a library (`put_onto_battlefield_as`, as a transforming card is): the creature
/// card stays in the library, the noncreature card enters.
#[test]
fn a_creature_card_put_onto_the_battlefield_from_a_library_stays_there() {
    let (mut game, table) = game();
    let bear = put(&mut game, table, &BEAR, Zone::library(YOU));
    let relic = put(&mut game, table, &RELIC, Zone::library(YOU));
    let mut cx = cx(&mut game);
    let bear_after = ops::put_onto_battlefield_as(&mut cx, bear, YOU, false, false, ZoneReason::Other, &[]).expect("nothing is asked");
    assert_eq!(bear_after, bear);
    assert!(cx.s().zone(Zone::library(YOU)).contains(&bear));
    let relic_after = ops::put_onto_battlefield_as(&mut cx, relic, YOU, false, false, ZoneReason::Other, &[]).expect("nothing is asked");
    assert!(cx.s().battlefield().contains(&relic_after));
}

/// An effect that has its controller play a land as it resolves (CR 305.1, 608.2g) can't play a land creature card from a
/// library, which can't enter; a land card that is no creature card it can.
#[test]
fn a_land_that_cannot_enter_cannot_be_played_as_an_effect_resolves() {
    let (mut game, table) = game();
    let arbor = put(&mut game, table, &ARBOR, Zone::library(YOU));
    let waste = put(&mut game, table, &WASTE, Zone::library(YOU));
    let caster = put(&mut game, table, &CASTER, Zone::STACK);
    let item = stack_item(StackKind::Spell, caster, AbilityRef { card: table.id_of(&CASTER), index: 0 }, YOU, &[]);
    game.state.stack_push(item);
    // The caster is resolving: its frame names it (`Cx::resolving`).
    let serial = game.state.stack.last().expect("the caster").serial;
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len() - 1;
    let resolving = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert!(!could_play_land_during_resolution(&resolving, arbor));
    assert!(could_play_land_during_resolution(&resolving, waste));
}
