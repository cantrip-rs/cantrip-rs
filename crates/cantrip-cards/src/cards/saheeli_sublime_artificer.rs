use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::SERVO;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

fn artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

/// "Another target artifact or creature you control": not the permanent the first target slot chose.
fn other_artifact_or_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.controller(id) == asker.controller
        && { let c = q.chars(id); c.is(CardType::Artifact) || c.is(CardType::Creature) }
        && !q.chosen_targets().contains(&t))
}

/// Target artifact you control becomes a copy of another target artifact or creature you control until end of turn, except it's
/// an artifact in addition to its other types (CR 707.9b). Both targets must still be legal.
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.target_legal(item, 0) || !g.target_legal(item, 1) {
        return Ok(());
    }
    let (Target::Object(artifact), Target::Object(copied)) = (item.target(0), item.target(1)) else { return Ok(()) };
    let values = g.copiable_values(copied).with_type(CardType::Artifact);
    g.become_copy(item.object, artifact, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(NONCREATURE_SPELL_YOU_CAST, cantrip_mechanics::tokens::create_a_servo, "Whenever you cast a noncreature spell, create a 1/1 colorless Servo artifact creature token."),
        ability(loyalty(-2), become_copy, "\u{2212}2: Target artifact you control becomes a copy of another target artifact or creature you control until end of turn, except it's an artifact in addition to its other types.")
            .targets(&[
                target(artifact_you_control),
                target(other_artifact_or_creature_you_control),
            ]),
    ],
    ..oracle::SAHEELI_SUBLIME_ARTIFICER
};

pub static EXTRA: &[&CardDef] = &[&SERVO];
