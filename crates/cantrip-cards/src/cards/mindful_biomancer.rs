use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::gain_life(g, item, 1), "When this creature enters, you gain 1 life."),
        ability(cost("{2}{G}"), effects::this_gets::<2, 2>, "{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.").once_each_turn(),
    ],
    ..oracle::MINDFUL_BIOMANCER
};
