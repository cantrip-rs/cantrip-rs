use cantrip_api::*;
use cantrip_mechanics::{effects, targets, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}{R}").and(sacrifice(1, targets::filters::land)), |g, item| effects::draw(g, item, 1), "{2}{R}, Sacrifice a land: Draw a card."),
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2} ({2}, Discard this card: Search your library for a Mountain card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::SEISMIC_MONSTROSAUR
};
