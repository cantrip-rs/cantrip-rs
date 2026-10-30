use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This creature can't be blocked by more than one creature" (CR 509.1b): the most blockers it may have is one.
    keywords: Keywords::of(&[Keyword::CantBeBlockedByMoreThanOne]),
    abilities: &[ability(cost("{5}"), effects::target_creature_gets::<2, 2>, "{5}: Target Elf you control gets +2/+2 until end of turn.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>)])],
    ..oracle::SAFEWRIGHT_CAVALRY
};
