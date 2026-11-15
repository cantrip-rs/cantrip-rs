use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[effects::enchanted_permanents_upkeep_damage_prevented_by_mana_paid(
        "At the beginning of the upkeep of enchanted enchantment's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.",
    )],
    enchant: Some(targets::ENCHANTMENT),
    granted_preventions: &[effects::UPKEEP_DAMAGE_PREVENTED_BY_MANA_PAID],
    ..oracle::POWER_LEAK
};
