use cantrip_api::*;
use cantrip_mechanics::triggers::this_or_another_of_subtype_you_control_enters;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// "You gain 1 life. If this is the second time this ability has resolved this turn, draw a card." The resolution count is per
/// source ability and turn: one earlier resolution this turn makes this the second, and the third and later draw nothing.
fn gain_life_and_draw_the_second_time(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_one_life(g, item)?;
    if g.history().resolutions(item.object, item.ability) == 1 {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Ally) }>),
        gain_life_and_draw_the_second_time,
        "Whenever this creature or another Ally you control enters, you gain 1 life. If this is the second time this ability has resolved this turn, draw a card.",
    )],
    ..oracle::SOUTH_POLE_VOYAGER
};
