use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature enters with a number of stun counters on it equal to three minus X. If X is 2 or less, it enters tapped"
/// (CR 614.1c-d, 122.1d): the X it was cast with; put onto the battlefield without being cast, X is 0 (CR 107.3). A number of
/// counters is never negative, so with X of 3 or more it gets none (ruling).
fn enters_stunned(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if entering.x < 3 {
        entering.counters[CounterKind::Stun as usize] += 3 - entering.x as i16;
    }
    if entering.x <= 2 {
        entering.tapped = true;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters: Some(enters_stunned), ..oracle::SLUMBERING_TRUDGE };
