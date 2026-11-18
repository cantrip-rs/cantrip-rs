use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS_OR_DIES};
use cantrip_oracle as oracle;

/// "When this artifact enters or is put into a graveyard from the battlefield, you draw a card and you lose 1 life" (CR 603.6a,
/// 603.10a, 700.4): one triggered ability that fires on either event, the second looked back on as the artifact last existed.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_DIES,
        effects::draw_a_card_and_lose_1_life,
        "When this artifact enters or is put into a graveyard from the battlefield, you draw a card and you lose 1 life.",
    )],
    ..oracle::MEPHITIC_DRAUGHT
};
