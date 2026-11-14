use cantrip_api::*;
use cantrip_mechanics::replacements::GAIN_THAT_MUCH_LIFE_PLUS_ONE;
use cantrip_mechanics::statics::{self, plus_pt, your_creatures_while_seven_above_starting_life};
use cantrip_oracle as oracle;

/// "If this card is in your opening hand, you may begin the game with it on the battlefield" (CR 103.6), "If you would gain
/// life, you gain that much life plus 1 instead" (CR 614.1a) and "As long as you have at least 7 life more than your starting
/// life total, creatures you control get +2/+2" (CR 613.4c, 103.4).
pub static CARD: CardDef = CardDef {
    opening_hand: Some(statics::BEGIN_ON_THE_BATTLEFIELD),
    replacements: &[GAIN_THAT_MUCH_LIFE_PLUS_ONE],
    statics: &[static_ability(
        Layer::PtModify,
        your_creatures_while_seven_above_starting_life,
        plus_pt::<2, 2>,
        "As long as you have at least 7 life more than your starting life total, creatures you control get +2/+2.",
    )],
    ..oracle::LEYLINE_OF_HOPE
};
