use cantrip_api::*;
use cantrip_mechanics::counter_target_ability;
use cantrip_oracle as oracle;

/// An activated ability from an artifact source, using last known characteristics if it left (CR 113.7a, 602.1).
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
    abilities: &[ability(
        cost("{1}{G}").and(TAP),
        counter_ability,
        "{1}{G}, {T}: Counter target activated ability from an artifact source.",
    ).targets(&[target(artifact_activated_ability)])],
    ..oracle::BROWN_OUPHE
};
