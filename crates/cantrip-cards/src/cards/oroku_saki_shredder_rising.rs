use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::sneak;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// "Sneak {1}{B}" (CR 702.190a): cast for {1}{B} and an unblocked attacker you control returned to its owner's hand, in your
/// declare blockers step, entering tapped and attacking (CR 702.190b). "Whenever Oroku Saki deals combat damage to a player,
/// you draw a card and lose 1 life": a player only, not a planeswalker or a creature.
pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{1}{B}"), "Sneak {1}{B}")],
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        effects::draw_a_card_and_lose_1_life,
        "Whenever Oroku Saki deals combat damage to a player, you draw a card and lose 1 life.",
    )],
    ..oracle::OROKU_SAKI_SHREDDER_RISING
};
