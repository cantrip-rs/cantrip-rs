use cantrip_api::*;
use cantrip_mechanics::replacements::{exile_if_it_would_die, EXILE_INSTEAD_OF_DYING};
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// Damage the target; if it is a creature, it can't regenerate and is exiled if it would die this turn (CR 611.2a, 614.1a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = g.state().target(item, 0);
    damage_target(g, item, 0, item.x);
    g.end_sentence();
    if let Target::Object(creature) = target
        && g.chars(creature).is(CardType::Creature)
    {
        instruction(g, |g| {
            g.add_effect(item.object, creature, EffectKind::CantBeRegenerated, Duration::EndOfTurn);
            exile_if_it_would_die(g, item, 0, creature);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.")],
    ..oracle::DISINTEGRATE
};
