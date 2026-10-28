use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::ARTIFACT_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(ARTIFACT_YOU_CONTROL_ENTERS, |g, item| effects::draw(g, item, 1), "Whenever an artifact you control enters, draw a card.")],
    ..oracle::FATEFUL_DISCOVERY
};
