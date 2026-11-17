use cantrip_api::*;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ATTACKS_WHILE_SADDLED};
use cantrip_mechanics::{double, effects, saddle, targets};
use cantrip_oracle as oracle;

/// "Double the number of +1/+1 counters on target creature" (CR 701.10e, 122.6): as many more as it has as the ability resolves, none for a
/// creature with none, nothing if the target is no longer legal (CR 608.2b).
fn double_the_counters_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    double::counters(g, &[creature], CounterKind::PlusOne);
    Ok(())
}

/// Both abilities target any creature, the Tumblewagg's controller's or not (CR 115.1). "Saddle 2" (CR 702.171a) is the shared mechanic;
/// "attacks while saddled" is judged as the Tumblewagg attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            effects::plus_one_counter_on_target,
            "At the beginning of combat on your turn, put a +1/+1 counter on target creature.",
        )
        .targets(&[targets::CREATURE]),
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            double_the_counters_on_target,
            "Whenever this creature attacks while saddled, double the number of +1/+1 counters on target creature.",
        )
        .targets(&[targets::CREATURE]),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::ORNERY_TUMBLEWAGG
};
