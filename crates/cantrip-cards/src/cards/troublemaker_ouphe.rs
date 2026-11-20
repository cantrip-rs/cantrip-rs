use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{bargain, effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, if it was bargained, exile target artifact or enchantment an opponent controls." An intervening
/// "if" (CR 603.4): `was_bargained` is checked as it enters and again as the ability resolves, and the target is chosen only when
/// the trigger is put on the stack (CR 603.3d), so an unbargained Ouphe, or a bargained one with no legal target, triggers nothing.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::exile_first_target,
        "When this creature enters, if it was bargained, exile target artifact or enchantment an opponent controls.",
    )
    .condition(bargain::was_bargained)
    .targets(&[targets::ARTIFACT_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS])],
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::TROUBLEMAKER_OUPHE
};
