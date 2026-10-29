use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, mill two cards" (CR 701.17).
fn mill_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef { abilities: &[triggered(THIS_ENTERS, mill_two, "When this creature enters, mill two cards.")], ..oracle::SCARBLADE_SCOUT };
