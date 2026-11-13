use cantrip_api::*;
use cantrip_mechanics::{exchange, targets, triggers};
use cantrip_oracle as oracle;

/// "Exchange control of target artifact or creature you control and target artifact or creature an opponent controls." (CR
/// 701.12a-b): both still legal as it resolves, or the exchange can't be completed and none of it happens.
pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL, targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS],
        exchange::control_of_two_targets,
    ),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::TRADE_THE_HELM
};
