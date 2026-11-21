use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::pay_life_for_either(Color::Green, Color::Blue, "{T}, Pay 1 life: Add {G} or {U}."),
        lands::sacrifice_to_draw(cost("{1}, {T}"), "{1}, {T}, Sacrifice Waterlogged Grove: Draw a card."),
    ],
    ..oracle::WATERLOGGED_GROVE
};
