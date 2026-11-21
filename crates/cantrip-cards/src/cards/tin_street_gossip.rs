use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_each(&[Mana::R, Mana::G]).only_for(ManaRestriction::FACE_DOWN_SPELLS_OR_TURNING_CREATURES_FACE_UP),
        "{T}: Add {R}{G}. Spend this mana only to cast face-down spells or to turn creatures face up.",
    )],
    ..oracle::TIN_STREET_GOSSIP
};
