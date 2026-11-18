use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature is turned face up, put a +1/+1 counter on each creature you control. Creatures you control gain trample
/// until end of turn." (CR 701.40b, 611.2c): the counters go on the creatures its controller has as it resolves, this one
/// included, and the trample goes to those same creatures, none that enters later.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        effects::plus_one_counter_on_each_creature_you_control_which_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>,
        "When this creature is turned face up, put a +1/+1 counter on each creature you control. Creatures you control gain trample until end of turn.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{5}{G}{G}")), "Disguise {5}{G}{G}")],
    ..oracle::GREENBELT_RADICAL
};
