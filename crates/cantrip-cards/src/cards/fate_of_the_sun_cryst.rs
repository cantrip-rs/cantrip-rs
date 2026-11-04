use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], effects::destroy_first_target),
    own_cost_changes: &[targets::costs_less_if_it_targets_a_tapped_creature::<2>],
    ..oracle::FATE_OF_THE_SUN_CRYST
};
