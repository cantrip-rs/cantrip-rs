use cantrip_api::*;
use cantrip_mechanics::replacements::{DOUBLE_TOKENS, DRAW_TWO_INSTEAD_EXCEPT_THE_FIRST_IN_YOUR_DRAW_STEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    replacements: &[
        // "If you would draw a card except the first one you draw in each of your draw steps, draw two cards instead."
        DRAW_TWO_INSTEAD_EXCEPT_THE_FIRST_IN_YOUR_DRAW_STEP,
        // "If one or more tokens would be created under your control, twice that many of those tokens are created instead."
        DOUBLE_TOKENS,
    ],
    ..oracle::BARD_KING_OF_DALE
};
