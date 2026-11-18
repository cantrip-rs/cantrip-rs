use cantrip_api::*;
use cantrip_mechanics::effects::{self, may_sacrifice_or_discard};
use cantrip_mechanics::select;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, you may sacrifice an artifact or discard a card. If you do, draw a card and this creature
/// gets +2/+0 until end of turn." The choice is made as the ability resolves and may be declined; the card and the pump come only
/// if the sacrifice or the discard actually happened (CR 608.2c-d, 701.9, 701.21).
fn sacrifice_or_discard_to_draw_and_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if may_sacrifice_or_discard(g, item.controller, item.object, CardType::Artifact, select::CARD)? {
        effects::draw(g, item, 1)?;
        effects::this_gets::<2, 0>(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        sacrifice_or_discard_to_draw_and_pump,
        "Whenever this creature attacks, you may sacrifice an artifact or discard a card. If you do, draw a card and this creature gets +2/+0 until end of turn.",
    )],
    labels: &["Sacrifice an artifact", "Discard a card", "Neither"],
    ..oracle::RECKLESS_DETECTIVE
};
