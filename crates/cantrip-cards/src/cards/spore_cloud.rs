use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// Tap the blockers, prevent combat damage this turn, and keep the current attackers and blockers
/// from untapping during their controllers' next untap steps (CR 701.26a, 615.1, 502.3).
fn spore_cloud(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let blockers: Vec<ObjectId> = g.state().battlefield().iter().copied()
        .filter(|&id| g.is_blocking(id) && g.chars(id).is(CardType::Creature))
        .collect();
    g.tap_together(&blockers);
    g.end_sentence();
    prevention::all_combat_damage_this_turn(g, item, 0);
    g.end_sentence();
    let combatants: Vec<ObjectId> = g.state().battlefield().iter().copied()
        .filter(|&id| (g.is_attacking(id) || g.is_blocking(id))
            && g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for &id in &combatants {
            g.add_effect(item.object, id, EffectKind::DoesntUntap, Duration::DuringItsControllersNextUntapStep);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], spore_cloud),
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt this turn.")],
    ..oracle::SPORE_CLOUD
};
