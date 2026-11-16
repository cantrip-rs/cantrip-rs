//! Two casts that differ only by the permission they are made by read apart (CR 601.3): a card that gives one way to cast
//! cards from a graveyard for each card type (Muldrotha, the Gravetide's kind) lets an artifact creature card be cast by
//! either, as the type that way names, so each row of its own alternative cost names the way, not only the permanent. Over
//! test cards.

use crate::actions::{describe, priority_options};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Caption Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// An artifact creature card with an alternative cost of its own, which a permission to cast it lets be paid (CR 118.9).
static TARGET: CardDef = CardDef {
    name: "Caption Test Construct",
    cost: generic(3),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ways: &[way(Cost::mana(generic(1)), ZoneKind::Hand, "Rush job {1}")],
    ..CardDef::BLANK
};

fn of_type(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId, t: CardType) -> Option<Cost> {
    let c = q.chars(card);
    (player == asker.controller && c.is(t)).then(|| Cost::mana(q.card(c.card).cost))
}

fn artifact_way(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    of_type(q, asker, card, player, CardType::Artifact)
}

fn creature_way(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    of_type(q, asker, card, player, CardType::Creature)
}

const ARTIFACT_TEXT: &str = "You may cast an artifact spell from your graveyard.";
const CREATURE_TEXT: &str = "You may cast a creature spell from your graveyard.";

/// One permission per type, from the graveyard (Muldrotha's kind).
static BY_TYPE: CardDef = CardDef {
    name: "Caption Test Gravetide",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_ways: &[
        GrantedWay { text: ARTIFACT_TEXT, cost: artifact_way, permission: true, ..GrantedWay::CAST },
        GrantedWay { text: CREATURE_TEXT, cost: creature_way, permission: true, ..GrantedWay::CAST },
    ],
    ..CardDef::BLANK
};

/// A single permission (Lurrus's kind): its row names the permanent.
static ONE_WAY: CardDef = CardDef {
    name: "Caption Test Dream-Den",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_ways: &[GrantedWay { text: CREATURE_TEXT, cost: creature_way, permission: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &TARGET, &BY_TYPE, &ONE_WAY])))
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

/// Your precombat main phase with three colorless mana in your pool, the construct in your graveyard and `source` on the
/// battlefield.
fn game(source: &'static CardDef) -> (Game, ObjectId) {
    let table = table();
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    let card = put(&mut game, &TARGET, Zone::graveyard(YOU));
    put(&mut game, source, Zone::BATTLEFIELD);
    let mana = game.state.zone(Zone::library(YOU))[0];
    game.state.add_mana(YOU, Mana::C, 3, mana, None, ManaLasts::Step, false);
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, card)
}

/// The rows of the casts offered of `card`, sorted.
fn rows(game: &mut Game, card: ObjectId) -> Vec<String> {
    let mut cx = Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    let options: Vec<ActionOption> = priority_options(&mut cx, YOU).into_iter().filter(|o| o.source == Some(card) && o.kind == ActionKind::CastSpell).collect();
    let mut rows: Vec<String> = options.iter().map(|o| describe(game.cards, &game.state, &game.log, o).into_owned()).collect();
    rows.sort();
    rows
}

/// Each of the two permissions casts the artifact creature for its mana cost and for its own alternative cost (CR 118.9),
/// and every row reads apart from the others: the alternative cost's row names the way it is cast by.
#[test]
fn casts_by_two_ways_of_one_card_read_apart() {
    let (mut game, card) = game(&BY_TYPE);
    let rows = rows(&mut game, card);
    assert_eq!(
        rows,
        sorted(vec![
            ARTIFACT_TEXT.to_string(),
            CREATURE_TEXT.to_string(),
            format!("Rush job {{1}} ({ARTIFACT_TEXT})"),
            format!("Rush job {{1}} ({CREATURE_TEXT})"),
        ])
    );
}

/// A card that gives one such way is named by the permanent, as before.
#[test]
fn a_cast_by_a_single_way_names_the_permanent() {
    let (mut game, card) = game(&ONE_WAY);
    assert_eq!(rows(&mut game, card), sorted(vec![CREATURE_TEXT.to_string(), "Rush job {1} (Caption Test Dream-Den)".to_string()]));
}

fn sorted(mut rows: Vec<String>) -> Vec<String> {
    rows.sort();
    rows
}
