use cantrip_api::*;
use cantrip_mechanics::{delayed, gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Exile target nontoken creature. If the gift wasn't promised, return that card to the battlefield under its owner's control
/// with a +1/+1 counter on it at the beginning of the next end step." (CR 603.7): the gift itself, a Fish for the opponent, was
/// given by the rules before this runs (CR 702.174j). Promised, the creature is only exiled; otherwise it is exiled and a delayed
/// ability is set up about the card, only if it is in exile.
fn exile_and_unless_gifted_return_later(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.gift.is_some() {
        cantrip_mechanics::exile_target(g, item, 0);
    } else {
        delayed::exile_target_then_return_at_next_end_step(g, item, 0, 0);
    }
    Ok(())
}

/// "Return that card to the battlefield under its owner's control with a +1/+1 counter on it" (CR 603.7c, 400.7, 122.6): the card
/// the delayed ability is about, if it is still in exile and still the card it was; each card of a merged permanent, together
/// (CR 730.3c, `exile_target_then_return_at_next_end_step`). It enters with the counter.
fn return_with_a_plus_one_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::return_exiled_with_counters(g, item, &[(CounterKind::PlusOne, 1)])
}

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONTOKEN_CREATURE], exile_and_unless_gifted_return_later),
    gift: Some(gift::TAPPED_FISH),
    delayed: &[delayed_trigger(return_with_a_plus_one_counter, "Return that card to the battlefield under its owner's control with a +1/+1 counter on it at the beginning of the next end step.")],
    ..oracle::PARTING_GUST
};
