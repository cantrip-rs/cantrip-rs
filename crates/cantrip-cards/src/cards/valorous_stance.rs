use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::target_creature_gains_indestructible, "Target creature gains indestructible until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::destroy_first_target, "Destroy target creature with toughness 4 or greater.").targets(&[targets::CREATURE_WITH_TOUGHNESS_4_OR_GREATER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::VALOROUS_STANCE };
