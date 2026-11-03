use cantrip_api::*;
use cantrip_mechanics::{attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block, exile_enchanted_creature}, blight, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{1}{W}").and(blight::cost(1)).and(SACRIFICE_THIS),
        exile_enchanted_creature,
        "{1}{W}, Blight 1, Sacrifice this Aura: Exile enchanted creature.",
    )],
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::SPIRAL_INTO_SOLITUDE
};
