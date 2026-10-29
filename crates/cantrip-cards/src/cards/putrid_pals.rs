use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Disappear \u{2014} This creature enters with two +1/+1 counters on it if a permanent left the battlefield under your control this
/// turn" (CR 614.1c, 122.6, 207.2c): judged as it enters, from the history, for the player it enters under. The counters are on it as
/// it enters, not put on it afterwards.
fn enters_with_two_counters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::permanent_left_battlefield_this_turn(g, entering.controller) {
        entering.counters[CounterKind::PlusOne as usize] += 2;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters: Some(enters_with_two_counters), ..oracle::PUTRID_PALS };
