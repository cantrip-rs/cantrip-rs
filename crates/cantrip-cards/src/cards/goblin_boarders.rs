use cantrip_api::*;
use cantrip_mechanics::history::creatures_attacked_with;
use cantrip_oracle as oracle;

/// "Raid - This creature enters with a +1/+1 counter on it if you attacked this turn" (CR 614.1c, 207.2c, 508.1): a replacement
/// effect, not a trigger, so it reads this turn's attacks as the creature enters under `entering.controller`.
fn enters_with_a_counter_if_you_attacked(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if creatures_attacked_with(g, entering.controller) > 0 {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_a_counter_if_you_attacked),
    ..oracle::GOBLIN_BOARDERS
};
