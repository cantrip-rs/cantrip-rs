use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, persist, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Remove any number of counters from target creature you control" (CR 122.1, 608.2d): the controller chooses which, of any
/// kinds, from none up to all of them (`effects::counters_on`).
fn remove_any_number_of_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let all = effects::counters_on(g, creature);
    g.remove_counters_up_to(item.controller, item.object, creature, all)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            delayed::first_target_gains_its_delayed_ability_until_end_of_turn,
            "When Rhys enters, another target creature you control gains persist until end of turn.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
        ability(
            cost("{W}").and(TAP),
            remove_any_number_of_counters,
            "{W}, {T}: Remove any number of counters from target creature you control. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed(),
    ],
    // The granted ability (the card's delayed ability 0).
    delayed: &[persist::trigger("Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)")],
    ..oracle::RHYS_THE_EVERMORE
};
