use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "All creatures get -2/-0 until end of turn." (CR 608.2, 611.2a).
fn all_creatures_get_minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_all_creatures(g, item.object, -2, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], all_creatures_get_minus_two),
    ..oracle::MARSH_GAS
};
