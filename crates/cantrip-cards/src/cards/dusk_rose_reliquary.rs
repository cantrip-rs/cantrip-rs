use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::triggers::{ward, THIS_ENTERS};
use cantrip_mechanics::exile_target_until_leaves_effect;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // As an additional cost to cast this spell, sacrifice an artifact or creature.
    additional_cost: Some(Cost::sacrifice(1, filters::artifact_or_creature)),
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(THIS_ENTERS, exile_target_until_leaves_effect, "When this artifact enters, exile target artifact or creature an opponent controls until this artifact leaves the battlefield.")
            .targets(&[targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::DUSK_ROSE_RELIQUARY
};
