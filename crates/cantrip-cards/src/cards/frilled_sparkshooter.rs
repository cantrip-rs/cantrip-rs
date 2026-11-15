use cantrip_api::*;
use cantrip_mechanics::history::an_opponent_lost_life_this_turn;
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it if an opponent lost life this turn" (CR 614.1c, 122.1, 119.3): a
/// replacement effect, not a trigger, so it reads the turn's life changes as the creature enters, from the event history:
/// an opponent who has lost any life this turn counts even if they gained it back, and only an opponent does.
fn enters_with_a_counter_if_an_opponent_lost_life(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if an_opponent_lost_life_this_turn(g, entering.controller) {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_a_counter_if_an_opponent_lost_life),
    ..oracle::FRILLED_SPARKSHOOTER
};
