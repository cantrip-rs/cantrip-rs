use cantrip_api::*;
use cantrip_mechanics::{history, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/0 green and blue Fractal creature token and put X +1/+1 counters on it, where X is the number of cards you've
/// drawn this turn" (CR 111.1, 122.1): X is worked out once, as this resolves (ruling 2026-03-20), from the history of draws
/// by the spell's controller alone. The counters are put on the token after it is made, so with X of 0 it is a 0/0 for a
/// moment and the state-based actions put it into the graveyard (CR 704.5f).
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = history::drawn_this_turn(g, item.controller) as i16;
    tokens::create_fractal_with_counters(g, item.controller, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], fractal),
    ..oracle::FRACTAL_ANOMALY
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
