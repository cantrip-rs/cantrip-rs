use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the
/// battlefield" (CR 610.3, 603.1): the card returns, a new object, when the enchantment leaves; if it has left before the ability
/// resolves, nothing is exiled at all (CR 610.3b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_target_until_leaves_effect,
        "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
    )
    .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS])],
    ..oracle::WEB_UP
};
