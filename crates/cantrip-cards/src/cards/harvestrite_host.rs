use cantrip_api::*;
use cantrip_mechanics::triggers::this_or_another_of_subtype_you_control_enters;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 until end of turn. Then draw a card if this is the second time this ability has resolved
/// this turn." The resolution count is per source ability and turn: one earlier resolution this turn makes this the second.
/// If the creature is no longer a legal target, the ability does not resolve and neither part happens (CR 608.2b).
fn pump_and_draw_the_second_time(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<1, 0>(g, item)?;
    g.end_sentence();
    if g.history().resolutions(item.object, item.ability) == 1 {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Rabbit) }>),
        pump_and_draw_the_second_time,
        "Whenever this creature or another Rabbit you control enters, target creature you control gets +1/+0 until end of turn. Then draw a card if this is the second time this ability has resolved this turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::HARVESTRITE_HOST
};
