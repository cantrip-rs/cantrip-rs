use cantrip_api::*;
use cantrip_mechanics::{effects, raid};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[raid::enters(|g, item| effects::draw(g, item, 1), "Raid \u{2014} When this creature enters, if you attacked this turn, draw a card.")],
    ..oracle::STORM_FLEET_SPY
};
