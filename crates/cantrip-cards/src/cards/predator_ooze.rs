use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, effects::plus_one_counter_on_this, "Whenever this creature attacks, put a +1/+1 counter on it."),
        triggered(
            triggers::CREATURE_DEALT_DAMAGE_BY_THIS_DIES,
            effects::plus_one_counter_on_this,
            "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::PREDATOR_OOZE
};
