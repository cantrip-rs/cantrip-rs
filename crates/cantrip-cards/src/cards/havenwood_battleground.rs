use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        mana_ability(
            TAP.and(SACRIFICE_THIS),
            add_n(Mana::G, 2),
            "{T}, Sacrifice this land: Add {G}{G}.",
        ),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::HAVENWOOD_BATTLEGROUND
};
