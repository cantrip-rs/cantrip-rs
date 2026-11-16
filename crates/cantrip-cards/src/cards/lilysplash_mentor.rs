use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{G}{U}"),
        effects::exile_target_then_return_with_a_plus_one_counter,
        "{1}{G}{U}: Exile another target creature you control, then return it to the battlefield under its owner's control with a +1/+1 counter on it. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    ..oracle::LILYSPLASH_MENTOR
};
