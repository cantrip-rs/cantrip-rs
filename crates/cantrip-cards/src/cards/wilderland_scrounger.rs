use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "...while you control a creature with power 4 or greater": asked as the attack is declared, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4; the ruling). The counters go on the creatures its
/// controller controls as it resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::plus_one_counter_on_each_creature_you_control,
        "Ferocious \u{2014} Whenever this creature attacks while you control a creature with power 4 or greater, put a +1/+1 counter on each creature you control.",
    )
    .only_if(ferocious::you_control_asked)],
    ..oracle::WILDERLAND_SCROUNGER
};
