use cantrip_api::*;
use cantrip_mechanics::triggers::creature_you_control_with_a_plus_one_counter_dies;
use cantrip_oracle as oracle;

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, creature_you_control_with_a_plus_one_counter_dies),
        draw_a_card,
        "Whenever a creature you control with a +1/+1 counter on it dies, draw a card.",
    )],
    ..oracle::MELTSTRIDER_EULOGIST
};
