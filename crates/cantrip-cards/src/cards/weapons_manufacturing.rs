use cantrip_api::*;
use cantrip_mechanics::triggers::{NONTOKEN_ARTIFACT_YOU_CONTROL_ENTERS, THIS_LEAVES};
use cantrip_mechanics::{damage_target, targets, tokens};
use cantrip_oracle as oracle;

/// "When this token leaves the battlefield, it deals 2 damage to any target." (CR 603.6c, 603.10a): the token is the
/// source, looked back on as it last was on the battlefield.
fn munitions_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

/// A colorless artifact token named Munitions.
pub static MUNITIONS: CardDef = CardDef {
    name: "Munitions",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(THIS_LEAVES, munitions_damage, "When this token leaves the battlefield, it deals 2 damage to any target.")
        .targets(&[targets::ANY])],
    text: "When this token leaves the battlefield, it deals 2 damage to any target.",
    ..CardDef::BLANK
};

fn make_munitions(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MUNITIONS, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&MUNITIONS];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONTOKEN_ARTIFACT_YOU_CONTROL_ENTERS,
        make_munitions,
        "Whenever a nontoken artifact you control enters, create a colorless artifact token named Munitions with \"When this token leaves the battlefield, it deals 2 damage to any target.\"",
    )],
    ..oracle::WEAPONS_MANUFACTURING
};
