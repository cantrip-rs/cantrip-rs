use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't block." (CR 604.1); flying is a core keyword and is kept beside it.
pub static CARD: CardDef = CardDef { keywords: Keywords::of(&[Keyword::Flying, Keyword::CantBlock]), ..oracle::VAMPIRE_INTERLOPER };
