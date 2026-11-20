use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, flash_cost, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "You may cast this spell as though it had flash if you pay {2} more to cast it" (CR 601.2b, 601.2f): the optional cost
/// `flash_cost` gives. "When this enchantment enters, exile target artifact or creature an opponent controls until this
/// enchantment leaves the battlefield" (CR 610.3, 603.1): the card returns, a new object, when the enchantment leaves; if it has
/// left before the ability resolves, nothing is exiled at all (CR 610.3b).
pub static CARD: CardDef = CardDef {
    optional_cost: Some(flash_cost::OPTIONAL_COST),
    labels: flash_cost::LABELS,
    abilities: &[triggered(
        THIS_ENTERS,
        exile_target_until_leaves_effect,
        "When this enchantment enters, exile target artifact or creature an opponent controls until this enchantment leaves the battlefield.",
    )
    .targets(&[targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::MYSTICAL_TETHER
};
