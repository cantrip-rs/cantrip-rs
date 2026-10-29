use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, your_creatures_with_plus_one_counters};
use cantrip_mechanics::{same_name, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it, where X is the number of differently named
/// lands you control": counted as the ability resolves, each land once unless its name is one already counted (CR 201.2b, ruling).
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut counted: Vec<ObjectId> = Vec::new();
    for id in g.permanents_of(item.controller) {
        if g.chars(id).is(CardType::Land) && !counted.iter().any(|&other| same_name(g, other, id)) {
            counted.push(id);
        }
    }
    let x = counted.len() as i16;
    let made = tokens::create(g, item.controller, &tokens::FRACTAL, 1)?;
    let puts: Vec<(ObjectId, CounterKind, i16)> = made.iter().map(|&token| (token, CounterKind::PlusOne, x)).collect();
    g.add_counters_together(&puts);
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
