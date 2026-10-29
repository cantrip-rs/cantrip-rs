use cantrip_api::*;
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER, REPARTEE};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature can't be blocked this turn"
/// (CR 509.1b): it takes no blocker not yet declared; a block already made stands (CR 509.1c-d, 509.2 only look at blocks as they
/// are declared).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            REPARTEE,
            effects::this_cant_be_blocked_this_turn,
            "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature can't be blocked this turn.",
        ),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            effects::draw_a_card_and_lose_1_life,
            "Whenever this creature deals combat damage to a player, you draw a card and lose 1 life.",
        ),
    ],
    ..oracle::SNOOPING_PAGE
};
