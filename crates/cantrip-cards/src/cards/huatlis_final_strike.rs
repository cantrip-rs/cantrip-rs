use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 until end of turn. It deals damage equal to its power to target creature an opponent
/// controls." (CR 608.2b, 608.2c, 120.3): the pump is done first, so the damage is the power it has now; with either target
/// illegal the creature deals no damage, and a spell whose every target is illegal does not resolve at all.
fn strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::pump_first_target_then_it_bites_second(g, item, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], strike),
    ..oracle::HUATLIS_FINAL_STRIKE
};
