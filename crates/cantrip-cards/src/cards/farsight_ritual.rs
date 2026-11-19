use cantrip_api::*;
use cantrip_mechanics::{
    bargain,
    library::{self, Rest},
};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. If this spell was bargained, look at the top eight cards of your library instead.
/// Put two of them into your hand and the rest on the bottom of your library in a random order." (CR 702.166b, 401.4, 609.3)
fn look_and_take_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let depth = if bargain::bargained(item) { 8 } else { 4 };
    library::look_at_top(g, item.controller, item.object, depth, (2, 2), false, |_| true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], look_and_take_two),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::FARSIGHT_RITUAL
};
