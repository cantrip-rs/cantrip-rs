use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Flying is a core keyword in the content JSON (CR 702.9). "Prowess, prowess" is two instances of the keyword
    // ability (CR 702.108a), so one noncreature spell triggers it twice and it gets +1/+1 twice (the reminder text).
    abilities: &[
        triggers::PROWESS,
        triggers::PROWESS,
        // "Whenever ... becomes the target of a spell or ability an opponent controls" (CR 603.2, 102.2, 102.3): the event
        // ward triggers on (CR 702.21a), once each time it becomes a target.
        triggered(
            triggers::THIS_BECOMES_THE_TARGET_OF_AN_OPPONENTS_SPELL_OR_ABILITY,
            |g, item| effects::draw(g, item, 1),
            "Whenever Ruric Thar becomes the target of a spell or ability an opponent controls, draw a card.",
        ),
    ],
    ..oracle::RURIC_THAR_BIOMAGUS
};
