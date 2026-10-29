use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, your_creatures_with_plus_one_counters};
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it, where X is the number of differently named
/// lands you control": counted as the ability resolves, the largest set of lands with no name in common (CR 201.2a-b, ruling).
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = lands::differently_named_lands(g, item.controller) as i16;
    tokens::create_fractal_with_counters(g, item.controller, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{G}").and(TAP),
        fractal,
        "{4}{G}, {T}: Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it, where X is the number of differently named lands you control.",
    )],
    statics: &[static_ability(Layer::Ability, your_creatures_with_plus_one_counters, have_trample, "Creatures you control with +1/+1 counters on them have trample.")],
    ..oracle::EMIL_VASTLANDS_ROAMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
