use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on that creature" (CR 603.6a, 122.1): the creature that entered, the trigger's subject. If it has left
/// the battlefield by now it is a new object (CR 400.7) and takes nothing (CR 608.2b).
fn counter_on_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object().filter(|&c| targets::permanent(g, c)) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        counter_on_that_creature,
        "Whenever another creature you control enters, put a +1/+1 counter on that creature.",
    )],
    ..oracle::GOOD_FORTUNE_UNICORN
};
