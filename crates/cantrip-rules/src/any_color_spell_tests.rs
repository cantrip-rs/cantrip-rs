//! "You may spend mana as though it were mana of any color to cast planeswalker spells" (CR 609.4b) judged before a card is
//! cast, as the spell it would be (CR 712.11c, 712.8a), over test cards: a modal double-faced card's planeswalker back face
//! is offered for green mana where its blue cost could not otherwise be paid, and its creature front face, of the same
//! cost, is not. No printed modal double-faced card in the engine has a planeswalker face (Valki, God of Lies's is not
//! in content); Oath of Nissa's scenarios hold the card cast as itself.
//!
//! A permission to cast a card that says as much of that spell (`PlayGrant::any_color`, Robber of the Rich's), offered for a
//! spell with {C} in its mana cost: no card in content has one, so Robber's scenario holds the {C} of a kicker, as it is paid.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

const BLUE: ManaCost = ManaCost { symbols: [0, 1, 0, 0, 0, 0], ..ManaCost::ZERO };

/// The back face, a planeswalker of its own cost, {U}.
static BACK: CardDef = CardDef {
    name: "Any Color Test Walker",
    types: TypeSet::of(&[CardType::Planeswalker]),
    cost: BLUE,
    loyalty: Some(3),
    ..CardDef::BLANK
};
/// The front face, a creature of the same cost: a modal double-faced card whose faces may each be cast (CR 712.11b).
static FRONT: CardDef = CardDef {
    name: "Any Color Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    cost: BLUE,
    power: Some(2),
    toughness: Some(2),
    back: Some(&BACK),
    ..CardDef::BLANK
};

/// Oath of Nissa's rule.
fn planeswalker_spells(q: &dyn Query, asker: &Asker, spending: &Spending) -> bool {
    spending.player == asker.controller
        && spending.purpose == SpendingFor::Casting
        && spending.source.is_some_and(|source| q.chars(source).is(CardType::Planeswalker))
}
static OATH: CardDef = CardDef {
    name: "Any Color Test Oath",
    types: TypeSet::of(&[CardType::Enchantment]),
    spend_any_color: &[planeswalker_spells],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The ways the card in your hand is offered in your main phase with {G} in your pool, with the permission on the
/// battlefield (yours) or without it: 0 the front face, `WAY_BACK_FACE` the back.
fn offered(oath: bool) -> Vec<u8> {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FRONT, &BACK, &OATH])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let card = game.state.push_obj(Obj::new(table.id_of(&FRONT), YOU, Zone::hand(YOU)));
    game.state.zone_push(Zone::hand(YOU), card);
    if oath {
        let id = game.state.push_obj(Obj::new(table.id_of(&OATH), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    game.state.update_player(YOU, |p| p.pool = ManaPool([0, 0, 0, 0, 1, 0]));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    let mut ways: Vec<u8> =
        priority_options(&mut cx(&mut game), YOU).iter().filter(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).map(|o| o.index).collect();
    ways.sort_unstable();
    ways
}

/// CR 609.4b, 712.11c: the permission is asked of each face as the spell it would be - the planeswalker back face may be cast
/// with green mana, the creature front face may not (`actions::casts_with_any_color`).
#[test]
fn a_permission_for_planeswalker_spells_reaches_a_planeswalker_back_face_and_not_its_creature_front() {
    assert_eq!(offered(false), Vec::<u8>::new(), "a face is offered for {{U}} with only {{G}} and no permission");
    assert_eq!(offered(true), [WAY_BACK_FACE], "the permission is not judged face by face");
}

/// A spell with {C} in its mana cost: {U}{C}.
static COLORLESS_SYMBOL: CardDef = CardDef {
    name: "Any Color Test Eldrazi",
    types: TypeSet::of(&[CardType::Artifact]),
    cost: ManaCost { symbols: [0, 1, 0, 0, 0, 1], ..ManaCost::ZERO },
    ..CardDef::BLANK
};

/// Whether the spell, exiled, is offered to you by a permission on these terms with `pool` to pay with.
fn offered_from_exile(grant: PlayGrant, pool: [u32; 6]) -> bool {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COLORLESS_SYMBOL])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let card = game.state.push_obj(Obj::new(table.id_of(&COLORLESS_SYMBOL), YOU, Zone::exile(YOU)));
    game.state.zone_push(Zone::exile(YOU), card);
    game.state.permission_push(PlayPermission { what: Playable::Card(card), player: YOU, grant, by: Grant { object: card, ability: AbilityRef::default() }, turn: 0 });
    game.state.update_player(YOU, |p| p.pool = ManaPool(pool));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    priority_options(&mut cx(&mut game), YOU).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
}

/// CR 609.4b, 105.2a: as though it were mana of any color, red pays {U} but not {C}, which no color stands for; colorless
/// mana pays either. Mana of any type pays both, and with neither term the {U} takes blue mana.
#[test]
fn a_permission_to_spend_mana_as_any_color_pays_no_colorless_symbol() {
    const RED_RED: [u32; 6] = [0, 0, 0, 2, 0, 0];
    const RED_COLORLESS: [u32; 6] = [0, 0, 0, 1, 0, 1];
    const COLORLESS_COLORLESS: [u32; 6] = [0, 0, 0, 0, 0, 2];
    let any_color = PlayGrant { any_color: true, ..PlayGrant::CAST };
    assert!(!offered_from_exile(any_color, RED_RED), "colored mana paid {{C}}");
    assert!(offered_from_exile(any_color, RED_COLORLESS), "colored mana did not pay {{U}} beside colorless mana for {{C}}");
    assert!(offered_from_exile(any_color, COLORLESS_COLORLESS), "colorless mana is spent as though it were any color");
    assert!(offered_from_exile(PlayGrant { any_type: true, ..PlayGrant::CAST }, RED_RED), "any type pays {{C}}");
    assert!(!offered_from_exile(PlayGrant::CAST, RED_COLORLESS), "the permission alone changed what pays {{U}}");
}
