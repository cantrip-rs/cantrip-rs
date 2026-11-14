use cantrip_api::*;
use cantrip_mechanics::fight::plus_one_counters_on_first_target_then_it_bites_second;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control. Then that creature deals damage equal to its power to target creature an
/// opponent controls." The counter is put first, so the damage is the power it has now (CR 608.2c, 120.3).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], plus_one_counters_on_first_target_then_it_bites_second::<1>),
    ..oracle::FELLING_BLOW
};
