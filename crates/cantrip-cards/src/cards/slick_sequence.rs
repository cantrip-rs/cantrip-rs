use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Slick Sequence deals 2 damage to any target. If you've cast another spell this turn, draw a card." (CR 120.3, 608.2): the second
/// sentence is judged as the spell resolves, from the event history of the turn (CR 601.2i): any other spell its controller has cast
/// this turn counts - one that was countered, one that is still on the stack - and this spell's own cast does not. With its only
/// target illegal the spell does nothing (CR 608.2b), so no card is drawn either.
fn damage_then_draw_if_cast_another_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    if g.history().casts_by(item.controller).any(|cast| cast.spell != item.object) {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], damage_then_draw_if_cast_another_spell), ..oracle::SLICK_SEQUENCE };
