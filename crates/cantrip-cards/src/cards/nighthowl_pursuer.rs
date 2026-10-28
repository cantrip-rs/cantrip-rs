use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::this_gets::<2, 2>,
        "Ferocious \u{2014} Whenever this creature attacks while you control a creature with power 4 or greater, this creature gets +2/+2 until end of turn.",
    )
    .only_if(cantrip_mechanics::ferocious::you_control_asked)],
    ..oracle::NIGHTHOWL_PURSUER
};
