use cantrip_api::*;
use cantrip_mechanics::{attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block}, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::PACIFISM
};
