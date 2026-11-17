use cantrip_api::*;
use cantrip_mechanics::{history, solved, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Each creature you control deals 1 damage to that creature": one sentence, so every creature deals its damage at once, each as
/// its own source (CR 120.3, 608.2c). The creatures that deal it are those its controller controls as the ability resolves. Its one
/// target being illegal there keeps it from resolving at all (CR 608.2b).
fn each_creature_you_control_deals_one_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = g.state().target(item, 0);
    let hits: Vec<(ObjectId, Target, u32)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.controller(id) == item.controller)
        .map(|id| (id, target, 1))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

/// "To solve - Three or more creatures attacked this turn" (CR 719.3a): three or more different creatures were declared as
/// attackers this turn. Only the active player declares attackers (CR 506.2, 508.1a), so it is how many creatures they attacked
/// with (`history::creatures_attacked_with`): a creature put onto the battlefield attacking was never declared, and a creature
/// that attacked and has left still did.
fn three_or_more_creatures_attacked_this_turn(q: &dyn Query, _: &Asker) -> bool {
    history::creatures_attacked_with(q, q.state().turn.active) >= 3
}

/// "Solved - Creatures you control get +1/+0" (CR 702.169b): the static ability's creatures, as long as the Case is solved.
fn your_creatures_while_solved(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    solved::solved_in_layers(q, source) && statics::your_creatures(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            each_creature_you_control_deals_one_damage,
            "When this Case enters, choose target creature you don't control. Each creature you control deals 1 damage to that creature.",
        )
        .targets(&[targets::CREATURE_YOU_DONT_CONTROL]),
        solved::to_solve(
            three_or_more_creatures_attacked_this_turn,
            "To solve \u{2014} Three or more creatures attacked this turn. (If unsolved, solve at the beginning of your end step.)",
        ),
    ],
    statics: &[static_ability(Layer::PtModify, your_creatures_while_solved, statics::plus_pt::<1, 0>, "Solved \u{2014} Creatures you control get +1/+0.")],
    ..oracle::CASE_OF_THE_GATEWAY_EXPRESS
};
