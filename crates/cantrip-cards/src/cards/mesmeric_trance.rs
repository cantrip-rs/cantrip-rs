use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{1}")), "Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        ability(cost("{U}").and(discard_cards(1)), |g, item| effects::draw(g, item, 1), "{U}, Discard a card: Draw a card."),
    ],
    ..oracle::MESMERIC_TRANCE
};
