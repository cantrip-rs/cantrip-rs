use cantrip_api::*;
use cantrip_mechanics::{damage_target, hand, targets};
use cantrip_oracle as oracle;

/// The card discarded to this activation is a land where it went, unless its characteristics are undefined (CR 701.9c).
fn if_discarded_card_was_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let land = g.history()
        .ability_cost_objects(item.serial, 0, CostPart::Discard(0))
        .filter_map(|card| g.obj(card).became)
        .any(|card| hand::discarded_chars(g, card).is_some_and(|chars| chars.is(CardType::Land)));
    if land {
        damage_target(g, item, 0, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            discard_cards(1),
            if_discarded_card_was_land,
            "Discard a card: If the discarded card was a land card, this enchantment deals 2 damage to target player or planeswalker. Any player may activate this ability.",
        )
        .targets(&[targets::PLAYER_OR_PLANESWALKER])
        .any_player_may_activate(),
    ],
    ..oracle::LANDS_EDGE
};
