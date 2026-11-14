use cantrip_api::*;
use cantrip_mechanics::{backup, effects, targets};
use cantrip_oracle as oracle;

/// Backup 2 (CR 702.165a): put two +1/+1 counters on target creature; if that's another creature, it gains the sacrifice
/// ability, fixed when backup triggers (CR 702.165d).
fn backup(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    backup::resolve(g, item, 2, &[], Some(0))
}

const BLAST: AbilityDef =
    ability(cost("{1}").and(SACRIFICE_THIS), effects::this_deals_damage_equal_to_its_power_to_first_target, "{1}, Sacrifice this creature: It deals damage equal to its power to any target.")
        .targets(&[targets::ANY]);

pub static CARD: CardDef = CardDef {
    abilities: &[
        backup::trigger("When this creature enters, put two +1/+1 counters on target creature. If that's another creature, it gains the following ability until end of turn.", backup),
        BLAST,
    ],
    delayed: &[BLAST],
    ..oracle::VOLDAREN_THRILLSEEKER
};
