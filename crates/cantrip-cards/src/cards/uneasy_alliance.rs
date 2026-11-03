use cantrip_api::*;
use cantrip_mechanics::{attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block, exile_enchanted_creature}, targets, tokens};
use cantrip_oracle as oracle;

/// "Exile enchanted creature. You create a 1/1 black Ninja creature token": the token is made whether or not the creature was
/// still there to exile.
fn exile_enchanted_creature_and_create_a_ninja(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_enchanted_creature(g, item)?;
    g.end_sentence();
    tokens::create(g, item.controller, &tokens::NINJA, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{5}").and(Cost { sacrifice_self: true, ..Cost::NONE }),
        exile_enchanted_creature_and_create_a_ninja,
        "{5}, Sacrifice this Aura: Exile enchanted creature. You create a 1/1 black Ninja creature token. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::UNEASY_ALLIANCE
};

pub static EXTRA: &[&CardDef] = &[&tokens::NINJA];
