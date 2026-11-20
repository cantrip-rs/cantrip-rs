use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}: Put a coin counter on this artifact. When you do, ..." (CR 122.6, 603.12): the counter goes on first, then the reflexive
/// ability is put on the stack with its target chosen then, so its mana value is judged against the count including this counter.
/// The counter is put on only while the Well is still there (CR 400.7); the reflexive ability triggers only when a counter was put on.
fn put_a_coin_counter_then_reflexive(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.obj(item.object).counters(CounterKind::Coin);
    g.add_counters(item.object, CounterKind::Coin, 1);
    if g.obj(item.object).counters(CounterKind::Coin) > before {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None);
    }
    Ok(())
}

/// "Target instant or sorcery card with mana value equal to the number of coin counters on this artifact from your graveyard"
/// (CR 115.1, 202.3, 603.12): the count is read as the target is chosen and again as the ability resolves (CR 608.2b).
fn instant_or_sorcery_with_mana_value_equal_to_coin_counters(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let coins = q.obj(asker.source).counters(CounterKind::Coin);
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Instant, CardType::Sorcery]) && matches!(t, Target::Object(id) if i32::try_from(q.mana_value(id)) == Ok(coins))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        put_a_coin_counter_then_reflexive,
        "{T}: Put a coin counter on this artifact. When you do, you may cast target instant or sorcery card with mana value equal to the number of coin counters on this artifact from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    delayed: &[delayed_trigger(
        effects::may_cast_first_target_free_exiling_it,
        "When you do, you may cast target instant or sorcery card with mana value equal to the number of coin counters on this artifact from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
    )
    .targets(&[target(instant_or_sorcery_with_mana_value_equal_to_coin_counters)])],
    ..oracle::WISHING_WELL
};
