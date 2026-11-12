use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Vehicle enters, exile target artifact or creature an opponent controls until this Vehicle leaves the battlefield"
/// (CR 610.3, 603.1): the card returns, a new object (CR 400.7), when the Vehicle leaves; nothing is exiled if it has left before
/// the ability resolves (CR 610.3b). "Crew 3" (CR 702.122a) and "Cycling {W}" (CR 702.29a) are the shared mechanics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this Vehicle enters, exile target artifact or creature an opponent controls until this Vehicle leaves the battlefield.",
        )
        .targets(&[targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS]),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
        triggers::cycling(mana("{W}"), "Cycling {W} ({W}, Discard this card: Draw a card.)"),
    ],
    ..oracle::DETENTION_CHARIOT
};
