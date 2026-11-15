use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 until end of turn. Then it deals damage equal to its power to target creature you don't
/// control." (CR 608.2b, 608.2c): the pump is done first, so the damage is the power it has now. If the creature you control is
/// still a legal target but the other is not, it still gets the pump and no damage is dealt; if the creature you control is
/// illegal, nothing happens (ruling 2024-07-26).
fn rabid_gnaw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::pump_first_target_then_it_bites_second(g, item, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], rabid_gnaw),
    ..oracle::RABID_GNAW
};
