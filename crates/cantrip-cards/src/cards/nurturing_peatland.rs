use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::pay_life_for_either(Color::Black, Color::Green, "{T}, Pay 1 life: Add {B} or {G}."),
        lands::sacrifice_to_draw(cost("{1}, {T}"), "{1}, {T}, Sacrifice Nurturing Peatland: Draw a card."),
    ],
    ..oracle::NURTURING_PEATLAND
};
