use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_TURNED_FACE_UP;
use cantrip_mechanics::{disguise, effects};
use cantrip_oracle as oracle;

/// "Creatures you control gain hexproof until end of turn. Untap them." (CR 702.11b, 701.26b, 611.2c): the creatures the controller has
/// as this resolves gain hexproof and are untapped together; one that enters later gains nothing.
fn hexproof_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Hexproof]).0 }>(g, item)?;
    effects::untap_all_creatures_you_control(g, item)
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{2}{W}")), "Disguise {2}{W}")],
    abilities: &[triggered(
        THIS_TURNED_FACE_UP,
        hexproof_and_untap,
        "When this creature is turned face up, creatures you control gain hexproof until end of turn. Untap them.",
    )],
    ..oracle::ESSENCE_OF_ANTIQUITY
};
