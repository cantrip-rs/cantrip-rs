use cantrip_api::*;
use cantrip_mechanics::{combat, targets};
use cantrip_oracle as oracle;

/// Untap and remove the attacking creature from combat, then gain control until end of turn (CR 506.4, 608.2c, 611.2a).
fn disharmony(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.untap(creature);
    g.remove_from_combat(creature);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::Control(item.controller), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_CREATURE], disharmony),
    cast_only_if: Some(combat::cast_only_before_blockers),
    ..oracle::DISHARMONY
};
