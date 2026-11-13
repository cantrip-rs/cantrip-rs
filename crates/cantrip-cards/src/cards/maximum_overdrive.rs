use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE],
        plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Deathtouch, Keyword::Indestructible]).0 }>,
    ),
    ..oracle::MAXIMUM_OVERDRIVE
};
