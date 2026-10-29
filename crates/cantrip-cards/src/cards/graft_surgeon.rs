use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it" (CR 614.1c, 122.6).
fn enter_with_counter(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += 1;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enter_with_counter),
    abilities: &[triggered(
        THIS_DIES,
        effects::put_its_counters_on_target,
        "When this creature dies, put its counters on up to one target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL.optional()])],
    ..oracle::GRAFT_SURGEON
};
