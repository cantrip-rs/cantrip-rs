use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::lands;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this artifact enters, draw a card." and "{1}, {T}: Add one mana of any color." (CR 605.1a): a filter ability that is a
/// mana ability, paid with one mana from the pool and the tap.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this artifact enters, draw a card."), lands::ANY_COLOR_FOR_ONE],
    ..oracle::PROPHETIC_PRISM
};
