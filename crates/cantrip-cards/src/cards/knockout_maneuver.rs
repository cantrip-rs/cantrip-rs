use cantrip_api::*;
use cantrip_mechanics::fight::plus_one_counters_on_first_target_then_it_bites_second;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], plus_one_counters_on_first_target_then_it_bites_second::<1>),
    ..oracle::KNOCKOUT_MANEUVER
};
