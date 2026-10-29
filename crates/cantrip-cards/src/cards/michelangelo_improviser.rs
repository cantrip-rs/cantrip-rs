use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{choose, select, sneak};
use cantrip_oracle as oracle;

/// "You may put a creature card and/or a land card from your hand onto the battlefield" (CR 608.2d, 110.2a, 101.4): up to one of
/// each, both chosen with the hand in view before either moves, and put onto the battlefield together, so each sees the other
/// enter. A card that is both (Dryad Arbor) can be the creature or the land but not both, so the land is chosen from the rest.
fn put_a_creature_and_or_a_land_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let mut chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, hand.clone(), select::creature_cards(select::up_to(1)))?;
    let rest: Vec<ObjectId> = hand.into_iter().filter(|card| !chosen.contains(card)).collect();
    chosen.extend(choose::select(g, you, item.object, Purpose::PutOntoBattlefield, rest, select::land_cards(select::up_to(1)))?);
    let entering: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|card| (card, you)).collect();
    g.put_all_onto_battlefield(&entering)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        put_a_creature_and_or_a_land_from_hand,
        "Whenever Michelangelo deals combat damage to a player, you may put a creature card and/or a land card from your hand onto the battlefield.",
    )],
    ways: &[sneak::way(mana("{2}{G}{G}"), "Sneak {2}{G}{G}")],
    ..oracle::MICHELANGELO_IMPROVISER
};
