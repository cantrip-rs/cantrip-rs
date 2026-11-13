use cantrip_api::*;
use cantrip_mechanics::{effects, raid};
use cantrip_oracle as oracle;

/// "Raid -- When this creature enters, if you attacked this turn, draw a card." (CR 207.2c, 603.4, 121.1). Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[raid::enters(|g, item| effects::draw(g, item, 1), "Raid \u{2014} When this creature enters, if you attacked this turn, draw a card.")],
    ..oracle::SKYSHIP_BUCCANEER
};
