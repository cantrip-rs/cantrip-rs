use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, create a 1/1 colorless Robot artifact creature token" (CR 603.6a, 111.1): the controller of the
/// ability creates it.
fn robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ROBOT_1_1, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, robot, "When this creature enters, create a 1/1 colorless Robot artifact creature token.")],
    ..oracle::MECHANIZED_NINJA_CAVALRY
};

pub static EXTRA: &[&CardDef] = &[&tokens::ROBOT_1_1];
