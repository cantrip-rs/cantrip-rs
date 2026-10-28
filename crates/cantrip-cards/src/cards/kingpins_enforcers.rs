use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}{B}, Sacrifice an artifact or creature: Draw a card." (CR 602.2, 701.21): the permanent is sacrificed as a cost, so
/// it can be Kingpin's Enforcers itself; its controller draws once the ability resolves. Lifelink is a core keyword read
/// from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}").and(sacrifice(1, targets::filters::artifact_or_creature)),
        |g, item| effects::draw(g, item, 1),
        "{2}{B}, Sacrifice an artifact or creature: Draw a card.",
    )],
    ..oracle::KINGPINS_ENFORCERS
};
