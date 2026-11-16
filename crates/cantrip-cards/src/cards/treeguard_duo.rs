use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{creatures_you_control, pump, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature you control gains vigilance and gets +X/+X, where X is the number of creatures you
/// control." (CR 611.2a, 608.2h): X is counted once, as the ability resolves, Treeguard Duo itself included if it is still
/// there, and does not change later in the turn (ruling 2024-07-26); nothing happens if the target is no longer legal
/// (CR 608.2b).
fn vigilance_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = creatures_you_control(g, item.controller) as i32;
    instruction(g, |g| {
        gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
        pump(g, item.object, creature, x, x);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        vigilance_and_grow,
        "When this creature enters, until end of turn, target creature you control gains vigilance and gets +X/+X, where X is the number of creatures you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::TREEGUARD_DUO
};
