use cantrip_api::*;
use cantrip_mechanics::statics::{itself_while_you_drew_two_or_more, plus_pt};
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flying is a core keyword read from the card's characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| tokens::create(g, item.controller, &tokens::CLUE, 1).map(|_| ()),
        "When this creature enters, create a Clue token. (It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_drew_two_or_more,
        plus_pt::<2, 0>,
        "This creature gets +2/+0 as long as you've drawn two or more cards this turn.",
    )],
    ..oracle::MESSENGER_HAWK
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
