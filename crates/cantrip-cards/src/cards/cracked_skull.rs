use cantrip_api::*;
use cantrip_mechanics::{
    effects, hand, select, targets,
    triggers::{ENCHANTED_CREATURE_DEALT_DAMAGE, THIS_ENTERS},
};
use cantrip_oracle as oracle;

/// "When this Aura enters, look at target player's hand. You may choose a nonland card from it. That player discards that card."
/// (CR 701.20e, 608.2d, 701.9a): the hand is looked at (not revealed) by the Aura's controller, who may choose one nonland card
/// or none; the player, who may be that controller, discards it, which a replacement can change. An all-land hand is still
/// looked at, the choice shown with nothing it may take; only an empty hand asks nothing. A target that is no longer legal ends
/// the ability (CR 608.2b).
fn look_and_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    if let Some(card) = hand::look_and_may_choose(g, item, player, Purpose::TakeFromHand, select::NONLAND_CARD)? {
        g.discard_together(&[card]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(
            THIS_ENTERS,
            look_and_discard,
            "When this Aura enters, look at target player's hand. You may choose a nonland card from it. That player discards that card.",
        )
        .targets(&[targets::PLAYER]),
        // Damage dealt to it by several sources at once is one batch (CR 603.2c): the one trigger.
        triggered(ENCHANTED_CREATURE_DEALT_DAMAGE, effects::destroy_subject, "When enchanted creature is dealt damage, destroy it.").sums_per_subject(),
    ],
    ..oracle::CRACKED_SKULL
};
