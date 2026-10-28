use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::{recruit, tokens, triggers};
use cantrip_oracle as oracle;

/// The slot for one opponent of "for each opponent, exile up to one target nonland permanent that player controls" (CR
/// 601.2c, 115.1): a nonland permanent that opponent controls. The core makes a slot of its own for each opponent
/// (`TargetSpec::EACH_OPPONENT`) and says which one this is.
fn nonland_permanent_the_slots_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::controlled_by_slot_opponent(q, id) && filters::nonland(q, asker, id)
}

const FOR_EACH_OPPONENT: TargetSpec =
    target(nonland_permanent_the_slots_opponent_controls).between(0, TargetSpec::EACH_OPPONENT).labelled("Nonland permanent {player} controls to exile");

/// "... exile up to one target nonland permanent that player controls until this enchantment leaves the battlefield" (CR
/// 610.3): the targets still legal when it resolves are exiled together, and return when the enchantment leaves; nothing is
/// exiled if it has already left (CR 610.3b).
fn exile_until_it_leaves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = targets::legal_objects(g, item);
    g.exile_with(&objects, g.linked(item), true);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            exile_until_it_leaves,
            "When this enchantment enters, for each opponent, exile up to one target nonland permanent that player controls until this enchantment leaves the battlefield.",
        )
        .targets(&[FOR_EACH_OPPONENT]),
        triggered(triggers::THIS_ENTERS, recruit::effect, "When this enchantment enters, recruit."),
    ],
    ..oracle::CELEBRATE_THE_MOUNTAIN_KING
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
