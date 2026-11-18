use cantrip_api::*;
use cantrip_mechanics::{top_of_library, library::{look_at_top, Rest}};
use cantrip_oracle as oracle;

/// Look at the top X cards of your library, where X is the number of lands you control. Put one of them into your
/// hand, or two if it was kicked, and the rest on the bottom of your library in a random order.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let lands = cantrip_mechanics::lands_you_control(g, player);
    let taken: usize = if g.obj(item.object).kicked > 0 { 2 } else { 1 };
    let taken = top_of_library(g, player, lands).len().min(taken) as u32;
    look_at_top(g, player, item.object, lands, (taken, taken), false, |_| true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    kicker: kicker(mana("{1}{U}")),
    ..oracle::CONSULT_THE_STAR_CHARTS
};
