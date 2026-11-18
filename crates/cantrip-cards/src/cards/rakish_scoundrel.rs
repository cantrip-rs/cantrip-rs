use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Deathtouch is a core keyword. "When this creature enters or is turned face up, target creature gains indestructible until end of
/// turn" (CR 603.6a, 702.168d, 702.12b): cast face down it has no abilities (CR 708.2), so only turning it face up triggers.
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{4}{B/G}{B/G}")), "Disguise {4}{B/G}{B/G}")],
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_TURNED_FACE_UP,
        effects::target_creature_gains_indestructible,
        "When this creature enters or is turned face up, target creature gains indestructible until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::RAKISH_SCOUNDREL
};
