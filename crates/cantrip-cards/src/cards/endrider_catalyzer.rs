use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_oracle as oracle;

/// "Max speed — {T}: Add {R}{R}" (CR 702.178a, 605.1a): the mana ability the creature is granted while its controller's speed
/// is 4, the first of the card's `delayed`.
pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    delayed: &[mana_ability(TAP, add_n(Mana::R, 2), "{T}: Add {R}{R}.")],
    statics: &[max_speed("Max speed \u{2014} {T}: Add {R}{R}.")],
    ..oracle::ENDRIDER_CATALYZER
};
