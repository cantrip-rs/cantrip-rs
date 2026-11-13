use cantrip_api::*;
use cantrip_mechanics::{effects, sacrifice_chosen, targets, triggers};
use cantrip_oracle as oracle;

/// The caster may sacrifice a land to prevent the spell being countered (CR 118.12, 701.6a).
fn counter_unless_sacrifice_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, sacrifice(1, targets::filters::land), &PAY_QUESTION)? {
        g.counter(item.object);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "sacrifice a land, or counter {subject}?",
    "",
    "Sacrifice a land",
    "Counter {subject}",
).no_default();

/// The active player sacrifices a land of their choice (CR 701.21a).
fn player_sacrifices_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    sacrifice_chosen(g, player, item.object, 1, |chars| chars.is(CardType::Land))?;
    Ok(())
}

/// This is a state trigger, not an intervening-if condition (CR 603.8).
fn no_lands(q: &dyn Query, _: &Asker) -> bool {
    !q.state().battlefield().iter().any(|&id| q.chars(id).is(CardType::Land))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::CAST_THIS_SPELL, counter_unless_sacrifice_land,
            "When you cast this spell, counter it unless you sacrifice a land."),
        triggered(triggers::EACH_PLAYERS_UPKEEP, player_sacrifices_land,
            "At the beginning of each player's upkeep, that player sacrifices a land of their choice."),
        state_triggered(no_lands, effects::sacrifice_self,
            "When there are no lands on the battlefield, sacrifice this enchantment."),
    ],
    ..oracle::MANA_VORTEX
};
