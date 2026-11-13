use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::{effects, mana, targets};
use cantrip_oracle as oracle;

/// "Another creature or Vehicle" (CR 109.1, 701.21a): a creature or Vehicle other than the source of the cost. The permanents
/// a cost can sacrifice are the paying player's own, so no controller check is needed.
fn another_creature_or_vehicle(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && targets::creature_or_vehicle(q, id)
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    // "This creature enters tapped." (CR 614.1d)
    enters: Some(mana::enters_tapped),
    delayed: &[ability(
        cost("{B}").and(sacrifice(1, another_creature_or_vehicle)),
        |g, item| effects::draw(g, item, 1),
        "{B}, Sacrifice another creature or Vehicle: Draw a card.",
    )],
    statics: &[max_speed("Max speed \u{2014} {B}, Sacrifice another creature or Vehicle: Draw a card.")],
    ..oracle::GAS_GUZZLER
};
