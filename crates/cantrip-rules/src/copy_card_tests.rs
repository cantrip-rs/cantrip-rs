//! A copy of an object created as a card (CR 707.12, 702.192a: "create a copy of this object in exile") copies the
//! object's copiable values (CR 707.2): its copy effects with their exceptions and its face-down values, not the card under
//! them. Over test cards: no printed card in the engine makes such a copy of an object that is itself a copy.

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static BEAST: CardDef = CardDef {
    name: "Copied Card Beast",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};
static SHAPESHIFTER: CardDef = CardDef { name: "Copied Card Shapeshifter", power: Some(0), toughness: Some(0), ..BEAST };

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAST, &SHAPESHIFTER])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, card: CardId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(card, YOU, zone));
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// A Shapeshifter that is a copy of the Beast "except its name is X and it has haste" is copied as that: the Beast with
/// those exceptions, not the Shapeshifter card.
#[test]
fn a_copy_in_exile_of_a_copy_has_its_copiable_values_and_exceptions() {
    let (mut game, table) = game();
    let shifter = put(&mut game, table.id_of(&SHAPESHIFTER), Zone::BATTLEFIELD);
    let values = Copiable::of(table.id_of(&BEAST)).named("Renamed Beast").with_keyword(Keyword::Haste);
    game.state.update_obj(shifter, |o| o.copy = Some(values));
    let mut cx = cx(&mut game);
    let copy = ops::copy_card_into(&mut cx, shifter, YOU, Zone::exile(YOU));
    assert!(cx.s().obj(copy).card_copy);
    assert_eq!(cx.s().obj(copy).zone, Zone::exile(YOU));
    let chars = cx.chars(copy);
    assert_eq!(chars.card, table.id_of(&BEAST));
    assert_eq!(chars.name, "Renamed Beast");
    assert!(chars.keywords.has(Keyword::Haste));
    assert_eq!(chars.power, Some(3));
}

/// A face-down permanent is copied as its face-down values (CR 707.2, 708.2a), not the card it is face down.
#[test]
fn a_copy_in_exile_of_a_face_down_permanent_has_its_face_down_values() {
    let (mut game, table) = game();
    let hidden = put(&mut game, table.id_of(&BEAST), Zone::BATTLEFIELD);
    game.state.update_obj(hidden, |o| {
        o.face_down = true;
        o.marks.face_down_as = Some(table.id_of(&SHAPESHIFTER));
    });
    let mut cx = cx(&mut game);
    let copy = ops::copy_card_into(&mut cx, hidden, YOU, Zone::exile(YOU));
    assert_eq!(cx.s().obj(copy).card, table.id_of(&SHAPESHIFTER));
    assert_eq!(cx.chars(copy).name, "Copied Card Shapeshifter");
}
