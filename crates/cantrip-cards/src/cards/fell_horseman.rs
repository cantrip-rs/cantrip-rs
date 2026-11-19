use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers::this_dies_about_the_card};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Return target creature card from your graveyard to your hand." Nothing is returned if the card is gone as it
/// resolves (CR 608.2b), and the spell is then not exiled on an adventure but goes to the graveyard.
pub static DEATHLY_RIDE: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_hand),
    ..oracle::DEATHLY_RIDE
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::ZONE_CHANGE, this_dies_about_the_card),
        library::put_subject_on_the_bottom,
        "When this creature dies, put it on the bottom of its owner's library.",
    )],
    adventure: Some(&DEATHLY_RIDE),
    ..oracle::FELL_HORSEMAN
};

pub static EXTRA: &[&CardDef] = &[&DEATHLY_RIDE];
