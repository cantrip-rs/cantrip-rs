use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: daybound and nightbound are the core's, read from the card's JSON, and the rest of its text is core keywords,
// so nothing is left to write.
pub static CARD: CardDef = CardDef { ..oracle::TIRELESS_HAULER };
