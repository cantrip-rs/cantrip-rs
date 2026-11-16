use cantrip_api::*;
use cantrip_mechanics::replacements::{plus_one_counters_apply, plus_one_counters_applies};
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::effects;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "{4}{W}: Until end of turn, if you would put one or more +1/+1 counters on a creature you control, put that many plus one +1/+1
/// counters on it instead" (CR 614.1a, 611.2c): each resolution gives its controller a replacement effect about the game, so it
/// outlasts Prairie Dog; each applies once (CR 614.5), so two resolutions add two.
fn until_end_of_turn_one_more_plus_one_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(item.controller, item.object, EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: 0 }, Duration::EndOfTurn);
    Ok(())
}

fn counters_on_your_creature(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    // "If you would put": the creature is yours (the shared test) and so is the putter, whoever it is (CR 122.6, 614.1a).
    matches!(proposal, Proposal::Counters(c) if c.putter == asker.controller)
        && plus_one_counters_applies(q, asker, proposal, CounterKind::PlusOne, &[CardType::Creature])
}

fn one_more_plus_one_counter(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    plus_one_counters_apply(proposal, CounterKind::PlusOne);
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_END_STEP,
            effects::plus_one_counter_on_this,
            "At the beginning of your end step, if you haven't cast a spell from your hand this turn, put a +1/+1 counter on this creature.",
        )
        .condition(history::you_havent_cast_a_spell_from_your_hand),
        ability(
            cost("{4}{W}"),
            until_end_of_turn_one_more_plus_one_counter,
            "{4}{W}: Until end of turn, if you would put one or more +1/+1 counters on a creature you control, put that many plus one +1/+1 counters on it instead.",
        ),
    ],
    granted_replacements: &[granted_replacement(
        ReplacementKind::Counters,
        counters_on_your_creature,
        one_more_plus_one_counter,
        "puts that many plus one +1/+1 counters instead",
    )
    .text("if you would put one or more +1/+1 counters on a creature you control, put that many plus one +1/+1 counters on it instead.")],
    ..oracle::PRAIRIE_DOG
};
