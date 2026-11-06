use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// Counter target creature spell (CR 701.6a, 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_SPELL], effect),
    ..oracle::REMOVE_SOUL
};
