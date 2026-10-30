use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_SPACECRAFT], library::owner_puts_first_target_on_top_or_bottom),
    own_cost_changes: &[targets::costs_less_if_it_targets_an_attacking_creature::<1>],
    ..oracle::GUIDANCE_FAILURE
};
