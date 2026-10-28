use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Target activated or triggered ability you control from a creature source" (CR 115.1, 113.7a): an ability on the stack
/// (a mana ability never is, CR 605.3b) that Echo's controller controls and whose source is a creature, as the source
/// last existed if it has left the battlefield since (its characteristics when the ability was put on the stack).
fn your_ability_from_a_creature_source(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Ability(serial) = t else { return false };
    q.state()
        .stack
        .iter()
        .any(|i| i.kind != StackKind::Spell && i.serial == serial && i.controller == asker.controller && q.chars(i.object).is(CardType::Creature))
}

/// "Copy target activated or triggered ability you control from a creature source. You may choose new targets for the copy."
/// (CR 707.10, 707.10c) The copy is neither activated nor triggered,; its only target no longer legal, the
/// ability does not resolve at all (CR 608.2b).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.target(0), item.controller, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}, {T}"),
        copy_it,
        "{1}, {T}: Copy target activated or triggered ability you control from a creature source. You may choose new targets for the copy.",
    )
    .targets(&[target(your_ability_from_a_creature_source)])],
    ..oracle::ECHO_PERCEPTIVE_PRODIGY
};
