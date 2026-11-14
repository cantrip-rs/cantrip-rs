use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, greatest_power, targets};
use cantrip_oracle as oracle;

/// "Prime Speaker Zegana enters with X +1/+1 counters on it, where X is the greatest power among other creatures you control"
/// (CR 614.1c, 122.1, 208.1): read as it enters, when it is not yet on the battlefield and so is not among the creatures
/// counted; no creature, or none with a power above 0, is X = 0 (CR 107.1b).
fn enters_with_counters_for_the_greatest_power(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += greatest_power::among_creatures_you_control(g, entering.controller) as i32;
    Ok(())
}

/// "Draw cards equal to its power": its power as the ability resolves, counters included, from last-known information if it
/// has left (CR 608.2h); none for a power of 0 or less (CR 107.1b).
fn draw_cards_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::power_or_last_known(g, item.object).max(0) as u32;
    effects::draw(g, item, cards)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters_for_the_greatest_power),
    abilities: &[triggered(THIS_ENTERS, draw_cards_equal_to_its_power, "When Prime Speaker Zegana enters, draw cards equal to its power.")],
    ..oracle::PRIME_SPEAKER_ZEGANA
};
