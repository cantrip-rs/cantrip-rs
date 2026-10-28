use cantrip_api::*;
use cantrip_mechanics::triggers::{subject_gets_plus_one, CREATURE_YOU_CONTROL_ATTACKS_ALONE};
use cantrip_oracle as oracle;

/// "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn." (CR 506.5, 611.2a): the creature
/// that attacked alone, whichever creature of its controller's it is; this one does not get it unless it is the one that
/// attacked. The ability is exalted's in all but name, so it shares its trigger and its effect, and it is no exalted ability.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        subject_gets_plus_one,
        "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.",
    )],
    ..oracle::AGENTS_OF_S_H_I_E_L_D
};
