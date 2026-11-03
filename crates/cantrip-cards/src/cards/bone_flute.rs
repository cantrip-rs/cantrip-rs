use cantrip_api::*;
use cantrip_mechanics::effects::pump_all_creatures;
use cantrip_oracle as oracle;

// All creatures on the battlefield as this resolves get -1/-0 until end of turn (CR 611.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_all_creatures(g, item.object, -1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}").and(TAP), effect, "{2}, {T}: All creatures get -1/-0 until end of turn.")],
    ..oracle::BONE_FLUTE
};
