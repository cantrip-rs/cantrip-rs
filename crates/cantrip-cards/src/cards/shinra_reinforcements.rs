use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Mill three cards and you gain 3 life." (CR 701.17a, 119.1): a library with fewer than three cards is milled in full, and the
/// life is gained all the same.
fn mill_three_and_gain_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 3);
    g.gain_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, mill_three_and_gain_3, "When this creature enters, mill three cards and you gain 3 life.")],
    ..oracle::SHINRA_REINFORCEMENTS
};
