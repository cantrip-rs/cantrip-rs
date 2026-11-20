use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Attacking creatures get +1/+0 and gain trample until end of turn." (CR 611.2a, 611.2c).
fn attacking_creatures_get_1_0_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::attacking_creatures_get_and_gain_trample(g, item.object, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], attacking_creatures_get_1_0_and_trample),
    ..oracle::STAMPEDE
};
