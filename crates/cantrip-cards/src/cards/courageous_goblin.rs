use cantrip_api::*;
use cantrip_mechanics::{effects, ferocious, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "...while you control a creature with power 4 or greater": asked as the attack is declared, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4; the ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::this_gets_and_gains_menace::<1, 0>,
        "Whenever this creature attacks while you control a creature with power 4 or greater, this creature gets +1/+0 and gains menace until end of turn.",
    )
    .only_if(ferocious::you_control_asked)],
    ..oracle::COURAGEOUS_GOBLIN
};
