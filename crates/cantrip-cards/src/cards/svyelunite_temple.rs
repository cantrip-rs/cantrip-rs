use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_n(Mana::U, 2),
            "{T}, Sacrifice this land: Add {U}{U}.",
        ),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::SVYELUNITE_TEMPLE
};
