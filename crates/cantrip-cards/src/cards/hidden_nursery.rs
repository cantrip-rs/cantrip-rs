use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::discover_land(
        Mana::G,
        "{T}: Add {G}.",
        "{4}{G}, {T}",
        "{4}{G}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery.",
    ),
    ..oracle::HIDDEN_NURSERY
};
