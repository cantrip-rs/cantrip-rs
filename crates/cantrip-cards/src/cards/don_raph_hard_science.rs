use cantrip_api::*;
use cantrip_mechanics::{affinity, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "The next noncreature spell you cast this turn": the first spell that is no creature uses it up, whatever was cast before.
fn noncreature(q: &dyn Query, spell: ObjectId) -> bool {
    !q.chars(spell).is(CardType::Creature)
}

/// "The next noncreature spell you cast this turn has affinity for artifacts" (CR 702.41a, 611.2c, 601.2f): the effect is
/// made for the player and lasts the turn, or until that spell is cast, whether or not this creature stays on the battlefield.
fn next_noncreature_spell_has_affinity(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.spells_cost_this_turn(item, item.controller, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        next_noncreature_spell_has_affinity,
        "Whenever Don & Raph attack, the next noncreature spell you cast this turn has affinity for artifacts. (It costs {1} less to cast for each artifact you control.)",
    )],
    lasting_cost_changes: &[LastingCostChange {
        text: "The next noncreature spell you cast this turn has affinity for artifacts.",
        change: affinity::for_artifacts,
        next: Some(noncreature),
    }],
    ..oracle::DON_RAPH_HARD_SCIENCE
};
