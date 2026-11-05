use cantrip_api::*;
use cantrip_mechanics::{empower, lands_you_control, statics::{grant_delayed_ability, your_planeswalkers}, targets, triggers};
use cantrip_oracle as oracle;

/// "Landfall -- Whenever a land you control enters, empower Jace 2." (CR 207.2c): the Jace token is made if its
/// controller has none, and the counters go on it (see `empower::jace`).
fn empower_jace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    empower::jace(g, item.controller, item.object, 2)
}

/// The granted "[-10]: Put a +1/+1 counter on target creature for each land you control." The lands are counted as the
/// ability resolves (CR 608.2h), "you" being the controller of the planeswalker that activated it (CR 113.6, 109.5).
/// One counter for each of them goes on the creature, which is still a legal target (CR 608.2b).
fn counters_for_each_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let lands = lands_you_control(g, item.controller);
    g.add_counters(creature, CounterKind::PlusOne, i64::from(lands as i16));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::LANDFALL, empower_jace, "Landfall \u{2014} Whenever a land you control enters, empower Jace 2.")],
    delayed: &[ability(
        loyalty(-10),
        counters_for_each_land,
        "\u{2212}10: Put a +1/+1 counter on target creature for each land you control.",
    )
    .targets(&[targets::CREATURE])],
    statics: &[static_ability(
        Layer::Ability,
        your_planeswalkers,
        grant_delayed_ability,
        "Planeswalkers you control have \"\u{2212}10: Put a +1/+1 counter on target creature for each land you control.\"",
    )],
    ..oracle::AVATAR_OF_BURGEONING_ECHOES
};

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
