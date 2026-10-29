use cantrip_api::*;
use cantrip_mechanics::{class, discard_chosen, effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this Class enters, mill two cards, draw two cards, then discard two cards."
fn loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.mill(you, 2);
    g.draw(you, 2);
    discard_chosen(g, you, item.object, 2)?;
    Ok(())
}

/// "Target artifact cards from your graveyard" (CR 115.1), up to two.
fn artifact_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
}

/// "Return up to two target artifact cards from your graveyard to your hand": the ones still legal, together.
fn return_artifacts(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returning = targets::legal_objects(g, item);
    g.return_to_hand(&returning);
    Ok(())
}

/// "At the beginning of combat on your turn", at level 3 (CR 716.2a): the ability exists only at that level, so it is
/// asked of the level as combat begins, and once triggered it resolves like any other.
fn combat_at_level_three(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::BeginCombat }
            if q.state().turn.active == q.controller(source) && class::level(q, source) >= 3 =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        class::level_two("{1}{U}", "{1}{U}: Level 2"),
        class::level_three("{4}{U}", "{4}{U}: Level 3"),
        triggered(THIS_ENTERS, loot, "When this Class enters, mill two cards, draw two cards, then discard two cards."),
        triggered(class::BECOMES_LEVEL_TWO, return_artifacts, "When this Class becomes level 2, return up to two target artifact cards from your graveyard to your hand.")
            .targets(&[up_to(2, artifact_card_in_your_graveyard)]),
        triggered(when(EventMask::STEP_BEGAN, combat_at_level_three), effects::three_counters_and_becomes_robot, "At the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. If it isn't a creature, it becomes a 0/0 Robot creature in addition to its other types.")
            .targets(&[targets::ARTIFACT_YOU_CONTROL]),
    ],
    ..oracle::DOES_MACHINES
};
