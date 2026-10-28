use cantrip_api::*;
use cantrip_mechanics::your_turn;
use cantrip_oracle as oracle;

/// "Target activated or triggered ability you control from an artifact source" (CR 115.1, 113.3): an ability on the stack
/// (a mana ability never is, CR 605.3b) that the asker controls, whose source is an artifact, as it last was if it has left
/// the battlefield (CR 113.7a).
fn your_ability_from_an_artifact(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Ability(serial) if q
        .state()
        .stack
        .iter()
        .any(|i| i.kind != StackKind::Spell && i.serial == serial && i.controller == asker.controller && q.chars(i.object).is(CardType::Artifact)))
}

/// "Copy target activated or triggered ability ... You may choose new targets for the copy." (CR 707.10, 707.10c)
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0) {
        g.copy_ability(item.target(0), item.controller, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(life(2), copy_it, "Pay 2 life: Copy target activated or triggered ability you control from an artifact source. You may choose new targets for the copy. Activate only during your turn and only once each turn.")
        .targets(&[target(your_ability_from_an_artifact)])
        .once_each_turn()
        .condition(your_turn)],
    ..oracle::SCIENTIST_SUPREME_OF_A_I_M
};
