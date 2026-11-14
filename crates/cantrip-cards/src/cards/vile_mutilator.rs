use cantrip_api::*;
use cantrip_mechanics::{each_opponent_sacrifices, targets, triggers};
use cantrip_oracle as oracle;

fn nontoken_enchantment(q: &dyn Query, id: ObjectId) -> bool {
    !q.state().obj(id).token && q.chars(id).is(CardType::Enchantment)
}

fn nontoken_creature(q: &dyn Query, id: ObjectId) -> bool {
    !q.state().obj(id).token && q.chars(id).is(CardType::Creature)
}

/// "Each opponent sacrifices a nontoken enchantment of their choice, then sacrifices a nontoken creature of their choice"
/// (CR 701.21a, 101.4): the whole first sacrifice happens before the second, so a creature that is also an enchantment can
/// be chosen for the first and is then not there for the second.
fn opponents_sacrifice_an_enchantment_then_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_sacrifices(g, item.controller, item.object, 1, nontoken_enchantment)?;
    g.end_sentence();
    each_opponent_sacrifices(g, item.controller, item.object, 1, nontoken_creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        opponents_sacrifice_an_enchantment_then_a_creature,
        "When this creature enters, each opponent sacrifices a nontoken enchantment of their choice, then sacrifices a nontoken creature of their choice.",
    )],
    // As an additional cost to cast this spell, sacrifice a creature or enchantment (CR 118.8, 601.2b).
    additional_cost: Some(sacrifice(1, targets::filters::creature_or_enchantment)),
    ..oracle::VILE_MUTILATOR
};
