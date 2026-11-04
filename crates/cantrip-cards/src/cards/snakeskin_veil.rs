use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Put a +1/+1 counter on target creature you control. It gains hexproof until end of turn. Both stop if the target is
/// no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_target_and_gain_until_end_of_turn(g, item, Keyword::Hexproof)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effect),
    ..oracle::SNAKESKIN_VEIL
};
