use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it if a creature died this turn" (CR 614.1c, 700.4): judged as it enters,
/// from the event history; any creature's death this turn counts, the opponents' and tokens' too, and it is not yet on the
/// battlefield, so it is not one of them.
fn enters_with_a_counter_if_a_creature_died(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::creatures_died_this_turn(g, None) > 0 {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters: Some(enters_with_a_counter_if_a_creature_died), ..oracle::CACKLING_SLASHER };
