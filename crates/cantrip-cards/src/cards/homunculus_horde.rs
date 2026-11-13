use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

/// "Whenever you draw your second card each turn, create a token that's a copy of this creature." (CR 121.2, 603.2, 707.2): the token
/// copies what this creature is as the ability resolves (its copiable values, as last it existed on the battlefield), none of its counters
/// or other effects; and it has this ability too.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        |g, item| copy::create_token_copy_of_it(g, item, |_, values| values),
        "Whenever you draw your second card each turn, create a token that's a copy of this creature.",
    )],
    ..oracle::HOMUNCULUS_HORDE
};
