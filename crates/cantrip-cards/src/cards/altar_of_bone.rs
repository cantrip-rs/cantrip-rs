use cantrip_api::*;
use cantrip_mechanics::{library, select, targets};
use cantrip_oracle as oracle;

// Search for a creature card, reveal it, put it into hand, then shuffle (CR 701.23, 701.20, 701.24).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::CREATURE_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], resolve),
    // Sacrifice exactly one creature as an additional cost (CR 118.8, 601.2h).
    additional_cost: Some(sacrifice(1, targets::filters::creature)),
    ..oracle::ALTAR_OF_BONE
};
