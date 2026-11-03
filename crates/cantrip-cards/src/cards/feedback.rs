use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP,
        |g, item| {
            g.damage(item.object, Target::Player(PlayerId(item.data[0] as u8)), 1);
            Ok(())
        },
        "At the beginning of the upkeep of enchanted enchantment's controller, this Aura deals 1 damage to that player.",
    )],
    enchant: Some(targets::ENCHANTMENT),
    ..oracle::FEEDBACK
};
