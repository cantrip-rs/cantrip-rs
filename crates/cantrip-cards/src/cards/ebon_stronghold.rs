use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_n(Mana::B, 2),
            "{T}, Sacrifice this land: Add {B}{B}.",
        ),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::EBON_STRONGHOLD
};
