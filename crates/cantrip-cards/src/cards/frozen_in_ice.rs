use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_object, attached_to_it, tap_attached},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "When this Aura enters, tap enchanted creature." The creature it is attached to as the trigger resolves (CR 303.4m,
/// 608.2h).
fn tap_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tap_attached(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    statics: &[loses_all_abilities(attached_object, "Enchanted creature loses all abilities.")],
    // "Enchanted creature ... can't become untapped" (CR 701.26b): a rule of the Aura's, not an ability of the creature
    // (CR 113.1a), so the creature losing all its abilities - by this Aura's first half, at once - leaves it in force
    // (CR 613.1f).
    cant_become_untapped: &[attached_to_it],
    ..oracle::FROZEN_IN_ICE
};
