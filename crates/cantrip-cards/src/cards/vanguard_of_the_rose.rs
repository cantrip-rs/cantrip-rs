use cantrip_api::*;
use cantrip_mechanics::effects::this_gains_indestructible_then_tap;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(sacrifice(1, filters::another_creature_or_artifact)),
        this_gains_indestructible_then_tap,
        "{1}, Sacrifice another creature or artifact: This creature gains indestructible until end of turn. Tap it.",
    )],
    ..oracle::VANGUARD_OF_THE_ROSE
};
