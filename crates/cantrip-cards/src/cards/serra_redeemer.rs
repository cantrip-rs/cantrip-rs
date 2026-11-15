use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "...put two +1/+1 counters on that creature." (CR 122.1, 122.6): the creature that entered, the one the trigger is about; if it has
/// left the battlefield by now nothing is put on it (CR 400.7).
fn two_counters_on_that_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        effects::counters_on_each(g, &[creature], CounterKind::PlusOne, 2);
    }
    Ok(())
}

/// "Whenever another creature you control with power 2 or less enters": the power is read as it enters (CR 603.6a), counters it entered with
/// included, and never again once the ability has triggered (the ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        two_counters_on_that_creature,
        "Whenever another creature you control with power 2 or less enters, put two +1/+1 counters on that creature.",
    )],
    ..oracle::SERRA_REDEEMER
};
