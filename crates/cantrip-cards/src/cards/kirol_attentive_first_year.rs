use cantrip_api::*;
use cantrip_mechanics::untapped_creature_you_control;
use cantrip_oracle as oracle;

/// "Target triggered ability you control" (CR 115.1): a triggered ability on the stack whose controller is the one activating.
/// An activated ability is not one, nor is a spell, nor an ability an opponent controls.
fn triggered_ability_you_control(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    matches!(target, Target::Ability(serial)
        if q.state().stack.iter().any(|i| i.serial == serial && i.kind == StackKind::Triggered && i.controller == asker.controller))
}

/// "Copy target triggered ability you control. You may choose new targets for the copy." (CR 707.10, 707.10c) Nothing if the
/// ability is no longer there (CR 608.2b).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.target(0), item.controller, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::tap_untapped(2, untapped_creature_you_control),
        copy_it,
        "Tap two untapped creatures you control: Copy target triggered ability you control. You may choose new targets for the copy. Activate only once each turn.",
    )
    .targets(&[target(triggered_ability_you_control)])
    .once_each_turn()],
    ..oracle::KIROL_ATTENTIVE_FIRST_YEAR
};
