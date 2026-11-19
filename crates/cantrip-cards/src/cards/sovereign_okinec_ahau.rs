use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "For each creature you control with power greater than that creature's base power, put a number of +1/+1 counters on that
/// creature equal to the difference" (CR 122.1, 208.1, 613.4): the creatures and their differences are read as it resolves,
/// before any counter lands, and the counters are put on all of them as one event (CR 101.4, 122.6).
fn counter_for_each_power_above_base(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut counters = Vec::new();
    for id in g.permanents_of(item.controller) {
        let chars = g.chars(id);
        if let (true, Some(power), Some(base)) = (chars.is(CardType::Creature) && chars.power_above_base(), chars.power, chars.base_power) {
            counters.push((id, CounterKind::PlusOne, power - base));
        }
    }
    g.add_counters_together(&counters);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            THIS_ATTACKS,
            counter_for_each_power_above_base,
            "Whenever Sovereign Okinec Ahau attacks, for each creature you control with power greater than that creature's base power, put a number of +1/+1 counters on that creature equal to the difference.",
        ),
    ],
    ..oracle::SOVEREIGN_OKINEC_AHAU
};
