use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    // "Max speed — {T}: Target creature gains haste until end of turn" (CR 702.178a): the ability exists only while its
    // controller's speed is 4, granted by the static below.
    delayed: &[ability(
        TAP,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Haste]).0 }>,
        "{T}: Target creature gains haste until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    statics: &[max_speed("Max speed \u{2014} {T}: Target creature gains haste until end of turn.")],
    ..oracle::AMONKHET_RACEWAY
};
