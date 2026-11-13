use cantrip_api::*;
use cantrip_mechanics::effects::creatures_you_control_get_and_gain;
use cantrip_mechanics::{lands, statics::subtype_spell};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and "you may cast Ally spells from the top of your
    // library" (CR 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopSpells(subtype_spell::<{ lands::sub(Subtype::Ally) }>, None)],
    abilities: &[ability(
        SACRIFICE_THIS,
        creatures_you_control_get_and_gain::<0, 5, { Keywords::of(&[Keyword::Indestructible]).0 }>,
        "Sacrifice Hakoda: Creatures you control get +0/+5 and gain indestructible until end of turn.",
    )],
    ..oracle::HAKODA_SELFLESS_COMMANDER
};
