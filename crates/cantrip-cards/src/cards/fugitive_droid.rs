use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// "Can't be blocked if an artifact entered the battlefield under your control this turn" (CR 509.1b): asked of the history,
/// which keeps each permanent as it entered (`Event::Entered`) - an artifact then, under that controller then, whatever it
/// has become since, wherever it has gone and whoever controls it now. Only the Droid's own attack is limited, and its
/// controller is the one it has as blockers are declared.
fn cant_be_blocked_if_an_artifact_entered_under_your_control(q: &dyn Query, source: ObjectId, _: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || !q.history().entered().any(|e| e.is(CardType::Artifact) && e.controller == q.controller(source))
}

/// "A spell that targets an artifact or creature you control" (CR 115.1, 115.9): a spell on the stack with at least one target
/// that is, on the battlefield, an artifact or a creature its Droid's controller controls. One that has left the battlefield is
/// none (CR 400.7), and the spell is no longer a legal target for the Droid; one that has only become an illegal target of
/// the spell still is what it is.
fn spell_that_targets_an_artifact_or_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(spell) = t else { return false };
    let s = q.state();
    s.stack.iter().any(|i| {
        i.kind == StackKind::Spell
            && i.object == spell
            && s.targets(i).iter().filter_map(|t| t.object()).any(|id| {
                targets::permanent(q, id) && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature)) && q.controller(id) == asker.controller
            })
    })
}

fn counter_that_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}").and(SACRIFICE_THIS),
        counter_that_spell,
        "{U}, Sacrifice this creature: Counter target spell that targets an artifact or creature you control.",
    )
    .targets(&[target(spell_that_targets_an_artifact_or_creature_you_control)])],
    block_rules: &[cant_be_blocked_if_an_artifact_entered_under_your_control],
    ..oracle::FUGITIVE_DROID
};
