use cantrip_api::*;
use cantrip_mechanics::{effects, your_turn};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b): checked as blockers are declared, with the
    // power the blocker has then.
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    abilities: &[ability(
        cost("{5}"),
        effects::plus_one_counter_on_this,
        "Waterbend {5}: Put a +1/+1 counter on this creature. Activate only during your turn. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)",
    )
    .waterbend()
    .condition(your_turn)],
    ..oracle::FOGGY_SWAMP_VINEBENDER
};
