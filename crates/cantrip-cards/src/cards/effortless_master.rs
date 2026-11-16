use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "This creature enters with two +1/+1 counters on it if you've cast two or more spells this turn" (CR 614.1c, 122.1): the
/// Master is itself one of the spells cast, so a second spell this turn is enough. The history holds every spell cast this
/// turn, countered ones too.
fn enters_with_counters_after_two_spells(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::cast_two_or_more_spells_this_turn(g.history(), entering.controller) {
        entering.counters[CounterKind::PlusOne as usize] += 2;
    }
    Ok(())
}

/// Vigilance and menace are core keywords read from its characteristics.
pub static CARD: CardDef = CardDef { enters: Some(enters_with_counters_after_two_spells), ..oracle::EFFORTLESS_MASTER };
