use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Lifelink, Keyword::CantBlock]),
    abilities: &[triggered(
        triggers::EERIE,
        effects::may_return_this_card_to_hand,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, you may return this card from your graveyard to your hand.",
    )
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::FEAR_OF_INFINITY
};
