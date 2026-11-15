use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::you_may_mill::<2>,
        "When this creature enters, you may mill two cards. (You may put the top two cards of your library into your graveyard.)",
    )],
    ..oracle::DAGGERFANG_DUO
};
