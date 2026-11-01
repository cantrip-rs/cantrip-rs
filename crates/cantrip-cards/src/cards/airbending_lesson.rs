use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, effects, targets};
use cantrip_oracle as oracle;

/// "Airbend target nonland permanent. Draw a card." (CR 701.65a): the permanent is exiled, then the controller draws. With the
/// target illegal the spell does not resolve and draws nothing (CR 608.2b).
fn airbend_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    airbend_targets(g, item)?;
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], airbend_then_draw),
    ..oracle::AIRBENDING_LESSON
};
