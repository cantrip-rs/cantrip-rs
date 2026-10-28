use cantrip_api::*;
use cantrip_mechanics::{lands, mana, targets};
use cantrip_oracle as oracle;

/// "This land enters tapped." (CR 614.1c), "{T}: Add {G} or {U}." and "{2}{G}{U}, {T}, Sacrifice this land: Put two +1/+1
/// counters on target Elf you control. Activate only as a sorcery." (CR 602.5d, 122.1), the last as the Hobbit cycle prints it
/// (`lands::sacrifice_for_two_counters`); "Elf" is read through the words of this card's text (CR 612).
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Green, Color::Blue]), "{T}: Add {G} or {U}."),
        lands::sacrifice_for_two_counters(
            "{2}{G}{U}, {T}",
            &[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>)],
            "{2}{G}{U}, {T}, Sacrifice this land: Put two +1/+1 counters on target Elf you control. Activate only as a sorcery.",
        ),
    ],
    ..oracle::ELVENKINGS_HALLS
};
