use cantrip_api::*;
use cantrip_mechanics::effects::may_sacrifice_an_artifact_or_discard_to_draw;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, you may sacrifice an artifact or discard a card. If you do, draw a card." (CR 608.2c-d): the
/// choice is made as the ability resolves; the card is drawn only if the sacrifice or the discard happened.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| may_sacrifice_an_artifact_or_discard_to_draw(g, item, 1),
        "When this creature enters, you may sacrifice an artifact or discard a card. If you do, draw a card.",
    )],
    labels: &["Sacrifice an artifact", "Discard a card", "Neither"],
    ..oracle::KUN_LUN_WARRIOR
};
