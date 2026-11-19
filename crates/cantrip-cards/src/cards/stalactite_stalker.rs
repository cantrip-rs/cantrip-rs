use cantrip_api::*;
use cantrip_mechanics::{effects, history, pump, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature gets -X/-X until end of turn, where X is this creature's power." The sacrifice is a cost (CR 601.2h), so
/// the Stalker is gone when this resolves and X is the power it last had on the battlefield, counters included (CR 113.7a,
/// 608.2h).
fn target_creature_gets_minus_this_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = targets::power_or_last_known(g, item.object).max(0);
    pump(g, item.object, creature, -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            effects::plus_one_counter_on_this,
            "At the beginning of your end step, if you descended this turn, put a +1/+1 counter on this creature.",
        )
        .condition(history::descended_this_turn),
        ability(
            cost("{2}{B}").and(SACRIFICE_THIS),
            target_creature_gets_minus_this_power,
            "{2}{B}, Sacrifice this creature: Target creature gets -X/-X until end of turn, where X is this creature's power.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::STALACTITE_STALKER
};
