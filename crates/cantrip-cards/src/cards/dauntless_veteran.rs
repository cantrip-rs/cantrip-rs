use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, creatures you control get +1/+1 until end of turn." The affected creatures are those its
/// controller controls as the ability resolves and no others (CR 611.2c), so a creature that enters afterwards is not pumped.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::creatures_you_control_get::<1, 1>, "Whenever this creature attacks, creatures you control get +1/+1 until end of turn.")],
    ..oracle::DAUNTLESS_VETERAN
};
