use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::DoesNotUntap]),
    abilities: &[triggered(
        triggers::EACH_END_STEP,
        effects::untap_self,
        "Morbid \u{2014} At the beginning of each end step, if a creature died this turn, untap this creature.",
    )
    .condition(history::a_creature_died_this_turn)],
    ..oracle::SLUMBERING_CERBERUS
};
