use cantrip_api::*;
use cantrip_mechanics::{attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block, exile_enchanted_creature}, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(cost("{2}{W}"), exile_enchanted_creature, "{2}{W}: Exile enchanted creature.")],
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::COOPED_UP
};
