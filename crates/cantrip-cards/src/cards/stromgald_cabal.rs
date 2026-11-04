use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// "{T}, Pay 1 life: Counter target white spell." (CR 608.2b, 701.6a).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(life(1)), counter, "{T}, Pay 1 life: Counter target white spell.")
        .targets(&[targets::spell_of_either_color::<{ Color::White as u8 }, { Color::White as u8 }>()])],
    ..oracle::STROMGALD_CABAL
};
