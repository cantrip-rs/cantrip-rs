use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{R}{R}").and(TAP),
        effects::gain_control_of_target_while_you_control_source,
        "{1}{R}{R}, {T}: Gain control of target artifact for as long as you control this creature.",
    )
    .targets(&[targets::ARTIFACT])],
    ..oracle::ALADDIN
};
