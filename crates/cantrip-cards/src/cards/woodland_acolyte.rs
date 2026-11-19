use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Mend the Wilds puts a permanent card from your graveyard on top of your library.
pub static MEND_THE_WILDS: CardDef = CardDef {
    spell: spell(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD], effects::put_first_target_on_top_of_library),
    ..oracle::MEND_THE_WILDS
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, draw a card.")],
    adventure: Some(&MEND_THE_WILDS),
    ..oracle::WOODLAND_ACOLYTE
};

pub static EXTRA: &[&CardDef] = &[&MEND_THE_WILDS];
