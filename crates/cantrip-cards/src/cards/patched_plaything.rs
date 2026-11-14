use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "This creature enters with two -1/-1 counters on it if you cast it from your hand" (CR 614.1c, 122.1): read off the history of its
/// cast, so a Plaything put onto the battlefield without being cast, cast from anywhere but its controller's hand, or the same card
/// returned after an earlier cast, enters without them.
fn enters_with_counters_if_cast_from_your_hand(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::cast_of(g, entering.object).is_some_and(|cast| cast.from == ZoneKind::Hand && cast.player == entering.controller) {
        entering.counters[CounterKind::MinusOne as usize] += 2;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters: Some(enters_with_counters_if_cast_from_your_hand), ..oracle::PATCHED_PLAYTHING };
