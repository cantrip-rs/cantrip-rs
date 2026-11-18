use cantrip_api::*;
use cantrip_mechanics::{
    attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block, enchanted_permanent_cant_activate},
    targets,
};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT_OR_CREATURE),
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    activation_rules: &[enchanted_permanent_cant_activate],
    ..oracle::PETRIFY
};
