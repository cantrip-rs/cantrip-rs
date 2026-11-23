use cantrip_api::*;
use cantrip_mechanics::{lands, mana, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(mana::add_for_each_subtype_you_control::<{ Mana::G as u8 }, { lands::sub(Subtype::Elf) }>),
        "{T}: Add {G} for each Elf you control.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        statics::plus_pt::<1, 1>,
        "Other Elf creatures you control get +1/+1.",
    )],
    ..oracle::ELVISH_ARCHDRUID
};
