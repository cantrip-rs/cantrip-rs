use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, each_opponent_sacrifices, greatest_power};
use cantrip_oracle as oracle;

const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 6"];

fn is_a_creature(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// "Each opponent sacrifices a creature of their choice. If evidence was collected, instead each opponent sacrifices a creature with
/// the greatest power among creatures they control." (CR 701.21a, 101.4, 608.2c): with no evidence, every opponent picks any
/// creature; with it, the replacement is the whole instruction, each opponent picking among the creatures tied for the greatest
/// power they control. Either way the choices are made on the board as it is and the sacrifices happen together.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.paid == 0 {
        each_opponent_sacrifices(g, item.controller, item.object, 1, is_a_creature)
    } else {
        let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
        greatest_power::sacrifice_one_with_the_greatest_power(g, item.object, &opponents)
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    optional_cost: Some(collect_evidence::optional(6)),
    labels: LABELS,
    ..oracle::EXTRACT_A_CONFESSION
};
