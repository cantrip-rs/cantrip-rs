use cantrip_api::*;
use cantrip_mechanics::counter_target_ability;
use cantrip_oracle as oracle;

/// An activated ability on the stack whose source is an artifact, using its last known characteristics if it left
/// (CR 113.7a, 602.1). Mana abilities do not use the stack (CR 605.3b).
fn artifact_activated_ability(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Ability(serial) = t else { return false };
    q.state().stack.iter().any(|i| {
        i.serial == serial && i.kind == StackKind::Activated && q.chars(i.object).is(CardType::Artifact)
    })
}

fn counter_ability(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target_ability(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_activated_ability)], counter_ability),
    ..oracle::RUST
};
