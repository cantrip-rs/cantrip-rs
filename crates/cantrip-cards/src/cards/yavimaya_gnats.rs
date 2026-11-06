use cantrip_api::*;
use cantrip_mechanics::regenerate;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{G}"), regenerate::this_effect, "{G}: Regenerate this creature."),
    ],
    ..oracle::YAVIMAYA_GNATS
};
