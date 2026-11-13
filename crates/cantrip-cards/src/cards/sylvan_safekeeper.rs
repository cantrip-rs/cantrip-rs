use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(Cost::sacrifice(1, targets::filters::land), effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Shroud]).0 }>, "Sacrifice a land: Target creature you control gains shroud until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SYLVAN_SAFEKEEPER
};
