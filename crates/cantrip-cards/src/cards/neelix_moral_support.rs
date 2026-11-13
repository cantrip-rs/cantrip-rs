use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_GAIN_LIFE};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Whenever you gain life, put a +1/+1 counter on Neelix" (CR 119.3, 603.2): one trigger for each life-gain event,
/// however much was gained (the token's own three life is one event).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When Neelix enters, create a Food token."),
        triggered(YOU_GAIN_LIFE, effects::plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on Neelix."),
    ],
    ..oracle::NEELIX_MORAL_SUPPORT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
