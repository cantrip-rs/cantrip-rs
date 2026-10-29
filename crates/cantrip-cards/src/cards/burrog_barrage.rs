use cantrip_api::*;
use cantrip_mechanics::{fight, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 until end of turn if you've cast another instant or sorcery spell this turn.
/// Then it deals damage equal to its power to up to one target creature an opponent controls." Another spell is any
/// instant or sorcery you cast this turn before this one (CR 601.2i); the power is read after the pump (CR 608.2c). With
/// the first target illegal nothing happens; with no second target, or an illegal one, only the pump does.
fn barrage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let instant_or_sorcery = |types: TypeSet| types.has(CardType::Instant) || types.has(CardType::Sorcery);
    if g.history().casts_by(item.controller).any(|c| c.spell != item.object && instant_or_sorcery(c.types)) {
        pump(g, item.object, creature, 1, 0);
    }
    g.end_sentence();
    fight::targets_bite(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS.optional()], barrage),
    ..oracle::BURROG_BARRAGE
};
