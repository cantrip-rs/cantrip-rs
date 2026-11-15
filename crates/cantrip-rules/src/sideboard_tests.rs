//! A card outside the game (CR 400.11a) is in no zone the rules cast or play a card from (CR 601.3, 305.1): only a way a
//! permanent gives that names the sideboard reaches one (CR 400.11b, 604.1), as The Holodeck's "you may play a permanent
//! card you own from outside the game" does. Over test cards.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// A land for the decks.
static FILLER: CardDef = CardDef { name: "Sideboard Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
/// A permanent card waiting outside the game: castable for {1} from a hand.
static OUTSIDE_CREATURE: CardDef = CardDef {
    name: "Sideboard Test Creature",
    cost: generic(1),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
/// A land card waiting outside the game.
static OUTSIDE_LAND: CardDef =
    CardDef { name: "Sideboard Test Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "You may play a permanent card you own from outside the game": the card's own mana cost for a spell, nothing for a land.
fn outside_the_game(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller {
        return None;
    }
    let c = q.chars(card);
    if c.is(CardType::Land) {
        return Some(Cost::NONE);
    }
    Some(Cost::mana(q.card(c.card).cost))
}

/// The way a permanent gives that names the sideboard (The Holodeck's kind).
static FROM_OUTSIDE: CardDef = CardDef {
    name: "Sideboard Test Holodeck",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_ways: &[GrantedWay {
        text: "outside the game",
        cost: outside_the_game,
        from: ZoneKind::Sideboard,
        lands: true,
        permission: true,
        ..GrantedWay::CAST
    }],
    ..CardDef::BLANK
};
/// The same way for the graveyard alone (`GrantedWay::CAST`'s own zone), which says nothing of a sideboard.
static FROM_GRAVEYARD: CardDef = CardDef {
    name: "Sideboard Test Graveyard Way",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_ways: &[GrantedWay { text: "from your graveyard", cost: outside_the_game, lands: true, permission: true, ..GrantedWay::CAST }],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &OUTSIDE_CREATURE, &OUTSIDE_LAND, &FROM_OUTSIDE, &FROM_GRAVEYARD])))
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

/// Your precombat main phase, a colorless mana in your pool, and a creature and a land of yours outside the game.
fn game() -> (Game, ObjectId, ObjectId) {
    let table = table();
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    let creature = put(&mut game, &OUTSIDE_CREATURE, Zone::sideboard(YOU));
    let land = put(&mut game, &OUTSIDE_LAND, Zone::sideboard(YOU));
    let source = game.state.zone(Zone::library(YOU))[0];
    game.state.add_mana(YOU, Mana::C, 1, source, None, ManaLasts::Step, false);
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, creature, land)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// The casts and land plays `player` is offered of `card`.
fn offers(game: &mut Game, player: PlayerId, card: ObjectId) -> Vec<ActionKind> {
    let mut cx = cx(game);
    priority_options(&mut cx, player)
        .into_iter()
        .filter(|o| o.source == Some(card) && matches!(o.kind, ActionKind::CastSpell | ActionKind::PlayLand))
        .map(|o| o.kind)
        .collect()
}

/// With nothing giving a way to it, a card outside the game is offered nothing, though the same card in a hand would be
/// cast or played: neither the casts of priority (CR 601.3) nor the land drop (CR 305.1) reach a sideboard.
#[test]
fn a_card_outside_the_game_is_neither_cast_nor_played() {
    let (mut game, creature, land) = game();
    assert!(offers(&mut game, YOU, creature).is_empty(), "nothing casts a card in a sideboard");
    assert!(offers(&mut game, YOU, land).is_empty(), "nothing plays a land in a sideboard");
    // The same cards in your hand are cast and played, so it is the zone that is offered nothing.
    let in_hand = put(&mut game, &OUTSIDE_CREATURE, Zone::hand(YOU));
    let land_in_hand = put(&mut game, &OUTSIDE_LAND, Zone::hand(YOU));
    assert_eq!(offers(&mut game, YOU, in_hand), vec![ActionKind::CastSpell]);
    assert_eq!(offers(&mut game, YOU, land_in_hand), vec![ActionKind::PlayLand]);
    // A way a permanent gives for the graveyard alone does not reach it either (CR 400.11b): it names its own zone.
    put(&mut game, &FROM_GRAVEYARD, Zone::BATTLEFIELD);
    assert!(offers(&mut game, YOU, creature).is_empty());
    assert!(offers(&mut game, YOU, land).is_empty());
}

/// The way a permanent gives that names the sideboard is the only thing that reaches it (CR 400.11b, 604.1): its controller
/// is offered the cast and the land play, and no one else is.
#[test]
fn only_the_way_that_names_the_sideboard_reaches_it() {
    let (mut game, creature, land) = game();
    put(&mut game, &FROM_OUTSIDE, Zone::BATTLEFIELD);
    assert_eq!(offers(&mut game, YOU, creature), vec![ActionKind::CastSpell]);
    assert_eq!(offers(&mut game, YOU, land), vec![ActionKind::PlayLand]);
    // The permanent is yours, so its way says nothing of an opponent's cards outside the game.
    assert!(offers(&mut game, THEM, creature).is_empty());
    assert!(offers(&mut game, THEM, land).is_empty());
}

/// A way that names the sideboard says nothing of the graveyard (CR 604.1): a land card there is not played by it, so a
/// permanent that gives one way for each zone (The Holodeck) offers that land once, by its graveyard way.
#[test]
fn the_way_that_names_the_sideboard_does_not_reach_the_graveyard() {
    let (mut game, _, _) = game();
    put(&mut game, &FROM_OUTSIDE, Zone::BATTLEFIELD);
    let land = put(&mut game, &OUTSIDE_LAND, Zone::graveyard(YOU));
    assert!(offers(&mut game, YOU, land).is_empty(), "the sideboard's way plays no land in a graveyard");
    put(&mut game, &FROM_GRAVEYARD, Zone::BATTLEFIELD);
    assert_eq!(offers(&mut game, YOU, land), vec![ActionKind::PlayLand], "the graveyard's way offers it once");
}

/// In a Commander game nothing outside the game is reached (CR 903.11), whatever way names it.
#[test]
fn nothing_outside_a_commander_game_is_reached() {
    let (mut game, creature, land) = game();
    game.state.commander = true;
    put(&mut game, &FROM_OUTSIDE, Zone::BATTLEFIELD);
    assert!(offers(&mut game, YOU, creature).is_empty());
    assert!(offers(&mut game, YOU, land).is_empty());
}
