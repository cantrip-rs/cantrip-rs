use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature you control gains indestructible until end of turn. If this spell was kicked, instead any number of
/// target creatures you control gain indestructible until end of turn." Each target still legal gains it (CR 608.2b); kicked
/// with no target chosen, it does nothing.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            cantrip_mechanics::effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // One target creature you control; kicked, any number of them (CR 702.33a, 601.2c).
    spell: spell(&[targets::CREATURE_YOU_CONTROL.if_paid(0, TargetSpec::ANY_NUMBER)], effect),
    kicker: kicker(mana("{2}{W}")),
    ..oracle::DIVINE_RESILIENCE
};
