use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::pay_life_for_either(Color::Red, Color::White, "{T}, Pay 1 life: Add {R} or {W}."),
        lands::sacrifice_to_draw(cost("{1}, {T}"), "{1}, {T}, Sacrifice Sunbaked Canyon: Draw a card."),
    ],
    ..oracle::SUNBAKED_CANYON
};
