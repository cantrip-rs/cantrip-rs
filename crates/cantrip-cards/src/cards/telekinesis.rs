use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// Tap the creature, prevent its combat damage this turn, and skip its controller's next two untaps (CR 608.2c, 615.1, 611.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.end_sentence();
    g.add_effect(item.object, creature,
        EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All },
        Duration::EndOfTurn);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::DoesntUntap, Duration::DuringControllersNextUntapSteps(2));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_preventions: &[prevention::COMBAT_DAMAGE_DEALT_BY.text("Prevent all combat damage that would be dealt by that creature this turn.")],
    ..oracle::TELEKINESIS
};
