use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::first_target_gets_and_gains_reach_until_end_of_turn_then_untap::<1, 3>),
    ..oracle::HIGH_STRIDE
};
