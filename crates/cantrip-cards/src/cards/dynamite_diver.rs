use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This creature saddles Mounts and crews Vehicles as though its power were 2 greater" is the two core keywords the
/// generated constant carries. "When this creature dies, it deals 1 damage to any target" (CR 120.3, 700.4): the creature
/// is the source, off its last-known information.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        effects::damage_first_target::<1>,
        "When this creature dies, it deals 1 damage to any target.",
    )
    .targets(&[targets::ANY])],
    ..oracle::DYNAMITE_DIVER
};
