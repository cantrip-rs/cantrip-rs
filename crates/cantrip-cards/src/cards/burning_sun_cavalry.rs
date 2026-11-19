use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_OR_BLOCKS;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// "...while you control a Dinosaur" (CR 205.3m) is asked as the creature attacks or blocks, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4): a Dinosaur that is gone by then changes nothing.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS_OR_BLOCKS,
        effects::this_gets::<1, 1>,
        "Whenever this creature attacks or blocks while you control a Dinosaur, this creature gets +1/+1 until end of turn.",
    )
    .only_if(statics::you_control_a::<{ lands::sub(Subtype::Dinosaur) }>)],
    ..oracle::BURNING_SUN_CAVALRY
};
