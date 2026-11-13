use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This creature can't block." (CR 604.1); flying is a core keyword and is kept beside it.
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::Flying, Keyword::CantBlock]),
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, return target creature card from your graveyard to your hand.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::VAMPIRE_SOULCALLER
};
