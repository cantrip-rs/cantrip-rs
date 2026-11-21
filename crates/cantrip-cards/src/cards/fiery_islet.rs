use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::pay_life_for_either(Color::Blue, Color::Red, "{T}, Pay 1 life: Add {U} or {R}."),
        lands::sacrifice_to_draw(cost("{1}, {T}"), "{1}, {T}, Sacrifice Fiery Islet: Draw a card."),
    ],
    ..oracle::FIERY_ISLET
};
