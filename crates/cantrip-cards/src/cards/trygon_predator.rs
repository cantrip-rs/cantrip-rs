use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature deals combat damage to a player, you may destroy target artifact or enchantment that player
/// controls" (CR 701.8, 603.3d, 608.2b): the target is chosen as the ability goes on the stack, among the damaged player's
/// artifacts and enchantments, and the "may" is asked as it resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        effects::may_destroy_first_target,
        "Whenever this creature deals combat damage to a player, you may destroy target artifact or enchantment that player controls.",
    )
    .targets(&[target(targets::artifact_or_enchantment_that_player_controls)])],
    ..oracle::TRYGON_PREDATOR
};
