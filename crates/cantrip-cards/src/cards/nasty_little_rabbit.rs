use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "...if you control a creature with power 4 or greater": an intervening "if" (CR 603.4), asked as combat begins and again as
/// the ability resolves (the ruling), so the +1/+1 counter is not put on if the creature is gone by then.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::plus_one_counter_on_this,
        "Ferocious \u{2014} At the beginning of combat on your turn, if you control a creature with power 4 or greater, put a +1/+1 counter on this creature.",
    )
    .condition(ferocious::you_control_asked)],
    ..oracle::NASTY_LITTLE_RABBIT
};
