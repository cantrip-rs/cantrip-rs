use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "Prevent all combat damage that would be dealt this turn." (CR 615.1, 611.2a).
fn prevent_combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::all_combat_damage_this_turn(g, item, 0);
    Ok(())
}

/// "Activate only before the combat damage step" of the first combat, even if damage was skipped (CR 506.8d–g).
fn before_combat_damage(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() < Step::CombatDamage
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::EndOfCombat }))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{G}{W}{U}").and(TAP), prevent_combat_damage,
        "{G}{W}{U}, {T}: Prevent all combat damage that would be dealt this turn. Activate only before the combat damage step.")
        .condition(before_combat_damage)],
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt this turn.")],
    ..oracle::ANGUS_MACKENZIE
};
