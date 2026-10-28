use cantrip_api::*;
use cantrip_mechanics::{effects, lands, pump, triggers};
use cantrip_oracle as oracle;

/// "Yellowjacket gets +1/+0 and gains lifelink until end of turn." (CR 611.2a): both effects are of the same ability, on
/// Yellowjacket if he is still on the battlefield.
fn pump_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // "Gets +1/+0 and gains lifelink" is one instruction (`instruction`): judged once both are done (CR 608.2c).
    instruction(g, |g| {
        pump(g, item.object, item.object, 1, 0);
        effects::gain_lifelink_until_end_of_turn(g, item.object, item.object);
    });
    Ok(())
}

/// Flying is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>),
        pump_and_lifelink,
        "Whenever another Villain you control enters, Yellowjacket gets +1/+0 and gains lifelink until end of turn.",
    )],
    ..oracle::YELLOWJACKET_HEARTLESS_MARAUDER
};
