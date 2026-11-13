use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Fuel the Flames deals 2 damage to each creature." Every creature on the battlefield as it resolves, all at once
/// (CR 120.3).
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], damage),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::FUEL_THE_FLAMES
};
