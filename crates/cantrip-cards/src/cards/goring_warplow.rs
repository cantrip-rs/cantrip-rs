use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: prototype is the core's, read from the card's JSON (`"prototype"`), and the rest of its text is core
// keywords (Deathtouch), so nothing is left to write.
pub static CARD: CardDef = CardDef { ..oracle::GORING_WARPLOW };
