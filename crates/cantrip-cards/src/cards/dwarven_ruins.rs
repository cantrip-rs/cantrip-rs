use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_n(Mana::R, 2),
            "{T}, Sacrifice this land: Add {R}{R}.",
        ),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::DWARVEN_RUINS
};
