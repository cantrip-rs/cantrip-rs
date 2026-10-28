use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while you control a creature with power 4 or greater, you gain 2 life": the power-4
/// condition is asked as the attack is declared, as part of the trigger event, and never again as the ability resolves (no
/// intervening "if", CR 603.4; the card's ruling). This creature counts itself when its power is 4 or more.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        |g, item| effects::gain_life(g, item, 2),
        "Ferocious \u{2014} Whenever this creature attacks while you control a creature with power 4 or greater, you gain 2 life.",
    )
    .only_if(ferocious::you_control_asked)],
    ..oracle::RAVENING_WARG
};
