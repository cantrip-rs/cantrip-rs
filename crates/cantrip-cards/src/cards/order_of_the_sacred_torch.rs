use cantrip_api::*;
use cantrip_mechanics::{self, targets};
use cantrip_oracle as oracle;

/// Counter the still-legal target spell (CR 608.2b, 701.6a).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(life(1)), counter, "{T}, Pay 1 life: Counter target black spell.")
        .targets(&[targets::spell_of_either_color::<{ Color::Black as u8 }, { Color::Black as u8 }>()])],
    ..oracle::ORDER_OF_THE_SACRED_TORCH
};
