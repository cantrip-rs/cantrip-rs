use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, empower, statics, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When Way of the Cryomancer enters, empower Jace 5."
fn empower_jace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    empower::jace(g, item.controller, item.object, 5)
}

/// "[−3]: When you next cast an instant or sorcery spell this turn, copy that spell": the loyalty ability sets up a delayed
/// triggered ability, number 1 of the card's `delayed` list, that triggers once on the next such spell its controller casts this
/// turn and ends with the turn if none is cast (CR 603.7b). Each activation sets up its own.
fn watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 1, item.object);
    Ok(())
}

const MINUS_THREE: &str =
    "\u{2212}3: When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.";

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, empower_jace, "When Way of the Cryomancer enters, empower Jace 5.")],
    delayed: &[
        ability(loyalty(-3), watch, MINUS_THREE),
        copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST,
    ],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_planeswalkers,
        statics::grant_delayed_ability,
        "Planeswalkers you control have \"\u{2212}3: When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.\"",
    )],
    ..oracle::WAY_OF_THE_CRYOMANCER
};

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
