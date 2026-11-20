use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Mill a card and put a bloodstain counter on this enchantment. Then sacrifice it if it has five or more bloodstain counters on it.
/// When you do, return target creature card from your graveyard to your hand" (CR 701.17a, 122.1, 608.2c, 603.12): the card is
/// milled first, so it may be the one returned, and the counter goes on only while the enchantment is still there (CR 400.7). It is sacrificed
/// as this ability resolves, and only a sacrifice that happened makes the reflexive ability, whose target is chosen as it goes on
/// the stack.
fn mill_a_card_count_a_bloodstain_then_sacrifice_at_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 1);
    g.add_counters(item.object, CounterKind::Bloodstain, 1);
    g.end_sentence();
    if g.obj(item.object).counters(CounterKind::Bloodstain) >= 5 && !g.sacrifice(item.controller, &[item.object]).is_empty() {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::damage_first_target::<3>, "When this enchantment enters, it deals 3 damage to target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            triggers::CREATURE_DIES,
            mill_a_card_count_a_bloodstain_then_sacrifice_at_five,
            "Whenever one or more creatures die, mill a card and put a bloodstain counter on this enchantment. Then sacrifice it if it has five or more bloodstain counters on it. When you do, return target creature card from your graveyard to your hand.",
        )
        .once_per_batch(),
    ],
    delayed: &[delayed_trigger(effects::return_first_target_to_hand, "When you do, return target creature card from your graveyard to your hand.").targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::BLOOD_SPATTER_ANALYSIS
};
