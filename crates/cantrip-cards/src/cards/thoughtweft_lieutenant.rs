use cantrip_api::*;
use cantrip_mechanics::triggers::this_or_another_of_subtype_you_control_enters;
use cantrip_mechanics::{lands, pump, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+1 and gains trample until end of turn."
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 1);
        trample_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Kithkin) }>),
        rally,
        "Whenever this creature or another Kithkin you control enters, target creature you control gets +1/+1 and gains trample until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::THOUGHTWEFT_LIEUTENANT
};
