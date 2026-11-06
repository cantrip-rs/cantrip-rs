//! The card's division uses the full creature count even beyond u16::MAX.
extern crate self as cantrip_oracle;
use cantrip_api::CardDef;

// The included card needs only its generated characteristics to compile; the test calls its actual arithmetic.
pub static DWARVEN_CATAPULT: CardDef = CardDef::BLANK;
#[allow(dead_code)]
#[path = "../../cantrip-cards/src/cards/dwarven_catapult.rs"]
mod card;

#[test]
fn creature_count_is_not_narrowed_before_division() {
    assert_eq!(card::damage_share(2, 65_537), 0);
    assert_eq!(card::damage_share(5, 3), 1);
    assert_eq!(card::damage_share(2, 1), 2);
}
