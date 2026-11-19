use cantrip_api::*;
use cantrip_mechanics::creatures_you_control;
use cantrip_mechanics::effects::{gain_until_end_of_turn, pump_creatures_you_control};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, creatures you control gain flying and get +X/+X until end of turn, where X is the number of
/// creatures you control." The creatures are the ones on the battlefield as this resolves and no creature that enters later
/// (CR 611.2c); X is counted once, as it resolves (CR 608.2h), Moonshaker Cavalry itself included if it is still there.
fn pump_and_fly(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creatures_you_control(g, item.controller) as i32;
    instruction(g, |g| {
        for creature in pump_creatures_you_control(g, item, x, x) {
            gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        pump_and_fly,
        "When this creature enters, creatures you control gain flying and get +X/+X until end of turn, where X is the number of creatures you control.",
    )],
    ..oracle::MOONSHAKER_CAVALRY
};
