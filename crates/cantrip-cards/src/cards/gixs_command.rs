use cantrip_api::*;
use cantrip_mechanics::{choose, effects};
use cantrip_mechanics::{greatest_power, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("PutCounters", "put two +1/+1 counters on a creature?", "It gains lifelink until end of turn", "Choose a creature", "No creature");

fn creatures(g: &dyn Query) -> Vec<ObjectId> {
    g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect()
}

/// Put two +1/+1 counters on up to one creature. It gains lifelink until end of turn. No target: with a creature on
/// the battlefield its controller is asked whether to, and then which creature.
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let candidates = creatures(g);
    if candidates.is_empty() {
        return Ok(());
    }
    if !cantrip_mechanics::effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let Some(Target::Object(chosen)) = choose::one_target(g, item.controller, item.object,
        candidates.iter().copied().map(Target::Object).collect(), "Choose a creature")? else { return Ok(()) };
    if targets::permanent(g, chosen) {
        g.add_counters(chosen, CounterKind::PlusOne, 2);
        cantrip_mechanics::effects::gain_lifelink_until_end_of_turn(g, item.object, chosen);
    }
    Ok(())
}

/// Destroy each creature with power 2 or less.
fn destroy_small(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let small: Vec<ObjectId> = creatures(g).into_iter().filter(|&id| g.chars(id).power.is_some_and(|p| p <= 2)).collect();
    g.destroy_all(&small);
    Ok(())
}

/// Each opponent sacrifices a creature with the greatest power among creatures they control: chosen by that
/// opponent, in turn order, when several tie; all of them sacrificed together.
fn sacrifice_greatest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
    greatest_power::sacrifice_one_with_the_greatest_power(g, item.object, &opponents)
}

static MODAL: ModalDef = modes(&[
    mode(counters, "Put two +1/+1 counters on up to one creature. It gains lifelink until end of turn."),
    mode(destroy_small, "Destroy each creature with power 2 or less."),
    mode(effects::return_up_to_two_creature_cards_from_your_graveyard_to_your_hand, "Return up to two creature cards from your graveyard to your hand."),
    mode(sacrifice_greatest, "Each opponent sacrifices a creature with the greatest power among creatures they control."),
])
    .choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GIXS_COMMAND };
