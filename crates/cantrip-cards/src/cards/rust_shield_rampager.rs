use cantrip_api::*;
use cantrip_mechanics::offspring;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b): checked as blockers are declared, with the
    // power the blocker has then. The offspring token copy has it too.
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)")],
    ..oracle::RUST_SHIELD_RAMPAGER
};
