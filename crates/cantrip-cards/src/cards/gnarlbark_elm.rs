use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{2}{B}").and(remove_counters_among(2, targets::filters::this_permanent)),
        effects::target_creature_gets::<-2, -2>,
        "{2}{B}, Remove two counters from this creature: Target creature gets -2/-2 until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE])
    .sorcery_speed()],
    ..oracle::GNARLBARK_ELM
};
