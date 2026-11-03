use cantrip_api::*;
use cantrip_mechanics::{attach::{enchanted_creature_cant_attack, enchanted_creature_cant_block, exile_enchanted_creature}, targets, tokens};
use cantrip_oracle as oracle;

/// "Exile enchanted creature. Create a 1/1 white Ally creature token": the token is made whether or not the creature was still
/// there to exile.
fn exile_enchanted_creature_and_create_an_ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_enchanted_creature(g, item)?;
    g.end_sentence();
    tokens::create_an_ally(g, item)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{5}").and(SACRIFICE_THIS),
        exile_enchanted_creature_and_create_an_ally,
        "{5}, Sacrifice this Aura: Exile enchanted creature. Create a 1/1 white Ally creature token. Activate only during your turn.",
    )
    .timing(Timing::OwnTurn)],
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::PATH_TO_REDEMPTION
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
