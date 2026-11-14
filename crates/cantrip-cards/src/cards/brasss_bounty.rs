use cantrip_api::*;
use cantrip_mechanics::{lands_you_control, tokens};
use cantrip_oracle as oracle;

/// "For each land you control, create a Treasure token" (CR 111.10a, 608.2h): the lands its controller has as it resolves,
/// through the layers, and all the Treasures are created at once.
fn create_a_treasure_for_each_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands = lands_you_control(g, item.controller) as u32;
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, lands, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], create_a_treasure_for_each_land), ..oracle::BRASSS_BOUNTY };

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
