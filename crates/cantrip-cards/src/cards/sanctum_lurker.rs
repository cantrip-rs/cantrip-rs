use cantrip_api::*;
use cantrip_mechanics::{effects, empower, statics::{grant_delayed_ability, your_planeswalkers}, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, empower Jace 1." (CR 701.71a): the Jace token is made if its controller has none, and the
/// counter goes on it (see `empower::jace`).
fn empower_jace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    empower::jace(g, item.controller, item.object, 1)
}

/// "Planeswalkers you control aren't put into their owners' graveyards for having 0 loyalty." (CR 704.5i): a planeswalker
/// its controller controls, judged as the state-based actions are checked.
fn yours(q: &dyn Query, asker: &Asker, planeswalker: ObjectId) -> bool {
    q.controller(planeswalker) == asker.controller
}

/// The granted "[+2]: This planeswalker deals 1 damage to each opponent and you gain 1 life." The planeswalker is the
/// source of the damage, as it last existed if it has left (CR 113.7a), and "you" is the controller of the ability
/// (CR 109.5).
fn ping_and_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, empower_jace, "When this creature enters, empower Jace 1.")],
    delayed: &[ability(loyalty(2), ping_and_gain, "+2: This planeswalker deals 1 damage to each opponent and you gain 1 life.")],
    statics: &[static_ability(
        Layer::Ability,
        your_planeswalkers,
        grant_delayed_ability,
        "Planeswalkers you control have \"[+2]: This planeswalker deals 1 damage to each opponent and you gain 1 life.\"",
    )],
    zero_loyalty_stays: &[yours],
    ..oracle::SANCTUM_LURKER
};

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
