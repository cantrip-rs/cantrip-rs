use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}").and(TAP), effects::loot, "{1}, {T}: Draw a card, then discard a card."),
        ability(
            cost("{7}").and(TAP).and(SACRIFICE_THIS),
            |g, item| effects::draw(g, item, 3),
            "{7}, {T}, Sacrifice this creature: Draw three cards. This ability costs {1} less to activate for each Town you control.",
        )
        .cost_change(cost_reduction::ability_less_for_each_subtype_you_control::<{ lands::sub(Subtype::Town) }>),
    ],
    ..oracle::QIQIRN_MERCHANT
};
