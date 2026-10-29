use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "Draw a card for each card type among spells you've cast this turn" (CR 205.2a, 601.2i, 121.1): each card type counted once
/// however many spells have it, a spell with several types giving each of them, from the types the spells had as they were cast.
/// A copy is not cast (CR 707.10) and a land played is not a spell, so neither counts.
fn draw_for_each_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let types = g.history().casts_by(item.controller).fold(TypeSet::NONE, |all, cast| cast.types.iter().fold(all, |all, t| all.with(t)));
    g.draw(item.controller, types.iter().count() as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        draw_for_each_type,
        "At the beginning of your end step, draw a card for each card type among spells you've cast this turn.",
    )],
    ..oracle::APRIL_ONEIL_HACKTIVIST
};
