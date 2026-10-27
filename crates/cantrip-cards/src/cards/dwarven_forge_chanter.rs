use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_life::<2>("Ward\u{2014}Pay 2 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays 2 life.)"),
        triggers::PROWESS,
    ],
    ..oracle::DWARVEN_FORGE_CHANTER
};
