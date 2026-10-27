use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: improvise is the core's, read from the card's JSON, and the rest of its text is core keywords,
// so nothing is left to write.
pub static CARD: CardDef = CardDef { ..oracle::SWEATWORKS_BRAWLER };
