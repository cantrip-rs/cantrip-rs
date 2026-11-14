use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers::{ward, THIS_ENTERS}};
use cantrip_oracle as oracle;

/// "Ward {2}" (CR 702.21a) is the shared mechanic. "Exile target ... until this enchantment leaves the battlefield" (CR 610.3,
/// 603.1): the card returns, a new object, when the enchantment leaves; if it has left before the ability resolves, nothing is
/// exiled at all (CR 610.3b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this enchantment becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this enchantment enters, exile target artifact, creature, or enchantment an opponent controls until this enchantment leaves the battlefield.",
        )
        .targets(&[targets::ARTIFACT_CREATURE_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::TRAPPED_IN_THE_SCREEN
};
