use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it if an opponent lost life this turn" (CR 614.1c, 122.1, 119.3): judged as it
/// enters, from the history, for the player it enters under. The counter is on it as it enters, not put on it afterwards.
fn enters_with_a_counter_if_an_opponent_lost_life(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::an_opponent_lost_life_this_turn(g, entering.controller) {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_a_counter_if_an_opponent_lost_life),
    abilities: &[ability(
        cost("{1}{B/R}"),
        |g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::Menace),
        "{1}{B/R}: This creature gains menace until end of turn.",
    )],
    ..oracle::CINDERING_CUTTHROAT
};
