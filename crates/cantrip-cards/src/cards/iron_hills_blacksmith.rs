use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a colorless Equipment artifact token named Axe with \"Equipped creature gets +1/+0\" and equip {2}."
fn create_axe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::AXE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_axe,
        "When this creature enters, create a colorless Equipment artifact token named Axe with \"Equipped creature gets +1/+0\" and equip {2}.",
    )],
    ..oracle::IRON_HILLS_BLACKSMITH
};

pub static EXTRA: &[&CardDef] = &[&tokens::AXE];
