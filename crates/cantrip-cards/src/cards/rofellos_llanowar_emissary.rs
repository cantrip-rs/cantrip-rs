use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(mana::add_for_each_subtype_you_control::<{ Mana::G as u8 }, { lands::sub(Subtype::Forest) }>),
        "{T}: Add {G} for each Forest you control.",
    )],
    ..oracle::ROFELLOS_LLANOWAR_EMISSARY
};
