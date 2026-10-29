use cantrip_api::*;
use cantrip_mechanics::{effects, increment, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 0/0 green and blue Fractal creature token and put X +1/+1 counters on it." (CR 111.1, 122.1, 107.3): with X of 0
/// the token is made and no counters are put, so it dies as a state-based action (CR 704.5f).
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_fractal_with_counters(g, item.controller, item.x as i16)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        increment::TRIGGER,
        triggered(
            triggers::PLUS_ONE_COUNTERS_PUT_ON_THIS,
            effects::add_one_mana_of_any_color,
            "Whenever one or more +1/+1 counters are put on Berta, add one mana of any color.",
        ),
        ability(cost("{X}").and(TAP), fractal, "{X}, {T}: Create a 0/0 green and blue Fractal creature token and put X +1/+1 counters on it."),
    ],
    ..oracle::BERTA_WISE_EXTRAPOLATOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
