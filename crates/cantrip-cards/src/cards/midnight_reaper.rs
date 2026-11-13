use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to you and you draw a card." (CR 120.3, 121.1, 608.2c): the damage is dealt by this creature as it
/// last existed (CR 608.2h) to the ability's controller, then they draw, in that order, even if the Reaper died in the same event.
fn deals_one_to_you_and_you_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::source_deals_one_to_controller(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        deals_one_to_you_and_you_draw,
        "Whenever a nontoken creature you control dies, this creature deals 1 damage to you and you draw a card.",
    )],
    ..oracle::MIDNIGHT_REAPER
};
