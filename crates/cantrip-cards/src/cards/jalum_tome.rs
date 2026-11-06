use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}").and(TAP),
            effects::loot,
            "{2}, {T}: Draw a card, then discard a card.",
        ),
    ],
    ..oracle::JALUM_TOME
};
