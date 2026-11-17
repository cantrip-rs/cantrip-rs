use cantrip_api::*;
use cantrip_mechanics::{delayed, disguise, tokens, triggers};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a): its controller creates a Clue token.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// Sets up the card's delayed ability 0 for the turn (CR 603.7b, 603.7d-e), this creature the source.
fn watch_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::this_turn(g, item, item.object);
    Ok(())
}

/// "When this creature is turned face up, until end of turn, whenever a creature you control deals combat damage to a player,
/// investigate." (CR 701.40b, 603.7b, 510.2): turning it face up sets up the delayed ability (the card's `delayed` ability 0) for the
/// rest of the turn, controlled by this ability's controller, who investigates once for each creature of theirs that deals combat damage
/// to a player (CR 603.2c), however the creatures got there.
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{1}{U}")), "Disguise {1}{U}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        watch_this_turn,
        "When this creature is turned face up, until end of turn, whenever a creature you control deals combat damage to a player, investigate.",
    )],
    delayed: &[triggered(
        when_asked(EventMask::DAMAGE, triggers::creature_you_control_deals_combat_damage_to_player),
        investigate,
        "Until end of turn, whenever a creature you control deals combat damage to a player, investigate.",
    )],
    ..oracle::MISTWAY_SPY
};
