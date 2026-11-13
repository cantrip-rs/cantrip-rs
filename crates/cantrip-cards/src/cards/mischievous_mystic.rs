use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

/// "Whenever you draw your second card each turn, create a 1/1 blue Faerie creature token with flying." (CR 121.2, 603.2, 111.1).
/// Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        tokens::create_a_faerie,
        "Whenever you draw your second card each turn, create a 1/1 blue Faerie creature token with flying.",
    )],
    ..oracle::MISCHIEVOUS_MYSTIC
};

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE];
