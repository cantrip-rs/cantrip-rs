//! A permanent as it entered the battlefield (CR 603.6a, 614.12) is in the history: `Event::Entered` keeps a last-known row,
//! which `history::Entered::chars` reads, so "a Hero entered under your control this turn" asks what it was then. A creature
//! that becomes another creature type after entering still reads as it entered; one that enters while the effect applies
//! reads with it. Test-only cards.

use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn becomes_hero(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.subtypes = SubtypeSet::of(&[Subtype::Hero]);
}

/// "Each creature is a Hero" (its other creature types gone, CR 205.1a), a Maskwood Nexus of its own.
static MASK: CardDef = CardDef {
    name: "Entered Test Mask",
    types: TypeSet::of(&[CardType::Artifact]),
    statics: &[static_ability(Layer::Type, creature, becomes_hero, "")],
    ..CardDef::BLANK
};
static GOBLIN: CardDef = CardDef {
    name: "Entered Test Goblin",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Goblin]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&MASK, &GOBLIN])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The entry of `permanent` this turn, as the history keeps it.
fn entry(game: &Game, permanent: ObjectId) -> history::Entered {
    game.history().entered().find(|e| e.permanent == permanent).expect("it entered this turn")
}

/// A Goblin entered, then the Mask: the Goblin is a Hero now, but entered a Goblin and no Hero. A second Goblin entering
/// with the Mask out entered a Hero.
#[test]
fn an_entry_reads_as_it_entered_whatever_it_has_become() {
    let (mut game, table) = game();
    let goblin = put(&mut game, table, &GOBLIN, Zone::hand(YOU));
    let mask = put(&mut game, table, &MASK, Zone::hand(YOU));
    let later = put(&mut game, table, &GOBLIN, Zone::hand(YOU));
    let mut cx = cx(&mut game);
    let goblin = ops::put_onto_battlefield(&mut cx, &[(goblin, YOU, false)], ZoneReason::Other).expect("nothing is asked")[0];
    ops::put_onto_battlefield(&mut cx, &[(mask, YOU, false)], ZoneReason::Other).expect("nothing is asked");
    let later = ops::put_onto_battlefield(&mut cx, &[(later, YOU, false)], ZoneReason::Other).expect("nothing is asked")[0];

    assert!(game.chars(goblin).has_subtype(Subtype::Hero), "the Mask makes it a Hero now");
    let first = entry(&game, goblin).chars(&game);
    assert!(first.has_subtype(Subtype::Goblin) && !first.has_subtype(Subtype::Hero), "it entered a Goblin");
    assert_eq!(entry(&game, goblin).controller, YOU);
    let second = entry(&game, later).chars(&game);
    assert!(second.has_subtype(Subtype::Hero) && !second.has_subtype(Subtype::Goblin), "it entered with the Mask out");
    assert!(entry(&game, later).is(CardType::Creature));
}

/// A Goblin and the Mask entering together: each entered seeing the other (CR 603.6a, 614.12), so the Goblin entered a Hero,
/// its row taken once both are there - whichever moved first.
#[test]
fn a_permanent_entering_with_the_mask_entered_with_it_applied() {
    for mask_first in [false, true] {
        let (mut game, table) = game();
        let goblin = put(&mut game, table, &GOBLIN, Zone::hand(YOU));
        let mask = put(&mut game, table, &MASK, Zone::hand(YOU));
        let both = if mask_first { [(mask, YOU, false), (goblin, YOU, false)] } else { [(goblin, YOU, false), (mask, YOU, false)] };
        let mut cx = cx(&mut game);
        let moved = ops::put_onto_battlefield(&mut cx, &both, ZoneReason::Other).expect("nothing is asked");
        let goblin = moved[if mask_first { 1 } else { 0 }];
        let entered = entry(&game, goblin).chars(&game);
        assert!(entered.has_subtype(Subtype::Hero) && !entered.has_subtype(Subtype::Goblin), "it entered with the Mask");
    }
}
