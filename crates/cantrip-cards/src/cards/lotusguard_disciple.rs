use cantrip_api::*;
use cantrip_mechanics::effects::{gain_indestructible_until_end_of_turn, gain_lifelink_until_end_of_turn};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature or Vehicle gains lifelink and indestructible until end of turn" (CR 611.2a, 702.15, 702.12b): both
/// keywords in one instruction, a layer 6 effect of this ability on the target if it is still a legal target as the ability
/// resolves (CR 608.2b). A Vehicle that is not a creature gains them too.
fn gain_lifelink_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        gain_lifelink_until_end_of_turn(g, item.object, target);
        gain_indestructible_until_end_of_turn(g, item.object, target);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, gain_lifelink_and_indestructible, "When this creature enters, target creature or Vehicle gains lifelink and indestructible until end of turn.")
        .targets(&[targets::CREATURE_OR_VEHICLE])],
    ..oracle::LOTUSGUARD_DISCIPLE
};
