use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{U}, {T}: Add {C}{C}{C}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(cost("{U}").and(TAP), add_n(Mana::C, 3), "{U}, {T}: Add {C}{C}{C}."),
    ],
    ..oracle::APPRENTICE_WIZARD
};
