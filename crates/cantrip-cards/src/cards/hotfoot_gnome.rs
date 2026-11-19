use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Haste]).0 }>,
        "{T}: Another target creature gains haste until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE])],
    ..oracle::HOTFOOT_GNOME
};
