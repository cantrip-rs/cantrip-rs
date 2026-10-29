use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, put a +1/+1 counter on target creature. You gain 1 life." The ability has one target:
/// if it is illegal as the ability resolves, neither instruction happens (CR 608.2b).
fn stabilize(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_target(g, item)?;
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        stabilize,
        "When this creature enters, put a +1/+1 counter on target creature. You gain 1 life.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::EMERGENCY_MEDICAL_HOLOGRAM
};
