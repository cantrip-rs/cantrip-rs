use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets};
use cantrip_oracle as oracle;

/// Destroy the land, then read its current or last-known snow status and controller (CR 608.2h, 701.8a).
fn destroy_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = destroy_target(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    if g.chars(land).supertypes.snow && g.chars(land).is(CardType::Land) {
        g.damage(item.object, Target::Player(g.controller(land)), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::LAND], destroy_land),
    ..oracle::ICEQUAKE
};
