use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::pay_life_for_either(Color::White, Color::Black, "{T}, Pay 1 life: Add {W} or {B}."),
        lands::sacrifice_to_draw(cost("{1}, {T}"), "{1}, {T}, Sacrifice Silent Clearing: Draw a card."),
    ],
    ..oracle::SILENT_CLEARING
};
