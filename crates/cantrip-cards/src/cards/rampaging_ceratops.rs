use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { keywords: Keywords::of(&[Keyword::CantBeBlockedExceptByThreeOrMore]), ..oracle::RAMPAGING_CERATOPS };
