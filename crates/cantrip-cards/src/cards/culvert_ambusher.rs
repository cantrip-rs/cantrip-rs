use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Disguise {4}{G} (CR 702.168). "When this creature enters or is turned face up, target creature blocks this turn if able": any creature
/// may be the target, this one included, and a creature cast face down triggers when it is turned up, not when it enters (CR 708.2).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_TURNED_FACE_UP,
        effects::target_creature_blocks_this_turn_if_able,
        "When this creature enters or is turned face up, target creature blocks this turn if able.",
    )
    .targets(&[targets::CREATURE])],
    ways: &[disguise::way(Cost::mana(mana("{4}{G}")), "Disguise {4}{G}")],
    ..oracle::CULVERT_AMBUSHER
};
