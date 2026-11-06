use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_n(Mana::W, 2),
            "{T}, Sacrifice this land: Add {W}{W}.",
        ),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::RUINS_OF_TROKAIR
};
