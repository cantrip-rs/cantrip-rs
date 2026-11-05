use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::set_first_target_base_pt::<1, 1>, "Target creature has base power and toughness 1/1 until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::set_first_target_base_pt::<4, 5>, "Target creature has base power and toughness 4/5 until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::PERFECTED_THEORY
};
