use cantrip_api::*;
use cantrip_mechanics::targets::{self, you_have_permanent_cards_in_graveyard};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

// Trample is the core's, read from the card's JSON.
/// "Descend 4 \u{2014} Whenever this creature attacks, if there are four or more permanent cards in your graveyard, put a +1/+1 counter on
/// target creature" (CR 207.2c: descend is an ability word; 603.4: the "if" is judged as it attacks, so the trigger is not put on the
/// stack with fewer, and again as it resolves).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        effects::plus_one_counter_on_target,
        "Descend 4 \u{2014} Whenever this creature attacks, if there are four or more permanent cards in your graveyard, put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE])
    .condition(you_have_permanent_cards_in_graveyard::<4>)],
    ..oracle::MALAMET_VETERAN
};
