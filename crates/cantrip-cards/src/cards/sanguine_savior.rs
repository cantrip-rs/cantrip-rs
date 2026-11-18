use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Flying and lifelink are core keywords. "When this creature is turned face up, another target creature you control gains lifelink
/// until end of turn" (CR 702.168d, 702.15, 611.2a): only turning it up triggers it - a Savior cast face up has no such event.
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{W/B}{W/B}")), "Disguise {W/B}{W/B}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Lifelink]).0 }>,
        "When this creature is turned face up, another target creature you control gains lifelink until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::SANGUINE_SAVIOR
};
