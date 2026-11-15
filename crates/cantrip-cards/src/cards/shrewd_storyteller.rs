use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Survival is an ability word (CR 207.2c). "Put a +1/+1 counter on target creature" (CR 122.1a): any creature, whoever
    // controls it, chosen as the trigger is put on the stack.
    abilities: &[triggers::survival(
        effects::plus_one_counter_on_target,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::SHREWD_STORYTELLER
};
