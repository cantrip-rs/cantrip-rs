use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

// Two nonartifact creatures, controlled by any player (CR 115.1a).
fn nonartifact_creature(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, target)
        && matches!(target, Target::Object(id) if !q.chars(id).is(CardType::Artifact))
}

// Exile the legal targets together, then deal 5 damage to the controller (CR 608.2b, 701.13, 120.3).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    g.end_sentence();
    g.damage(item.object, Target::Player(item.controller), 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(nonartifact_creature).between(2, 2)], resolve),
    ..oracle::ASHES_TO_ASHES
};
