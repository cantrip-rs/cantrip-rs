use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b): checked as blockers are declared, with the
    // power the blocker has then, and no matter what happens to it afterwards.
    keywords: Keywords::of(&[Keyword::Reach, Keyword::CantBeBlockedByPowerTwoOrLess]),
    abilities: &[triggered(
        triggers::THIS_BECOMES_THE_TARGET_OF_AN_OPPONENTS_SPELL_OR_ABILITY,
        |g, item| effects::draw(g, item, 1),
        "Whenever this creature becomes the target of a spell or ability an opponent controls, draw a card.",
    )],
    ..oracle::OLD_FAT_SPIDER
};
