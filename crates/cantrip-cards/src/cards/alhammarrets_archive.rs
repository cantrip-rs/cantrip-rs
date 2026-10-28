use cantrip_api::*;
use cantrip_mechanics::replacements::{DRAW_TWO_INSTEAD_EXCEPT_THE_FIRST_IN_YOUR_DRAW_STEP, GAIN_TWICE_THAT_MUCH_LIFE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    replacements: &[
        // "If you would gain life, you gain twice that much life instead."
        GAIN_TWICE_THAT_MUCH_LIFE,
        // "If you would draw a card except the first one you draw in each of your draw steps, draw two cards instead."
        DRAW_TWO_INSTEAD_EXCEPT_THE_FIRST_IN_YOUR_DRAW_STEP,
    ],
    ..oracle::ALHAMMARRETS_ARCHIVE
};
