use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more creatures you control become tapped" (CR 701.26a, 603.2c): a creature under this enchantment's
/// controller's control, tapped by any means; the creatures tapped together (attackers declared, a cost) trigger it once
/// (`once_per_batch`).
fn creature_you_control_becomes_tapped(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent } if q.controller(*permanent) == me.controller && q.chars(*permanent).is(CardType::Creature) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Draw a card, then discard a card and put a plan counter on this enchantment." (CR 701.9, 122.6) The counter goes only on
/// the enchantment that triggered, if it is still there (CR 400.7).
fn loot_and_plan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::loot(g, item)?;
    g.add_counters(item.object, CounterKind::Plan, 1);
    Ok(())
}

/// "Target instant and/or sorcery card in your graveyard" (CR 115.1): a card of either type in the graveyard of the ability's
/// controller, still that card (CR 400.7).
fn instant_or_sorcery_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Instant, CardType::Sorcery])
}

/// "Return up to two target instant and/or sorcery cards from your graveyard to your hand": those still legal go together
/// (CR 608.2b, 101.4).
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::legal_objects(g, item);
    g.return_to_hand(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::TAPPED, creature_you_control_becomes_tapped),
            loot_and_plan,
            "Whenever one or more creatures you control become tapped, draw a card, then discard a card and put a plan counter on this enchantment.",
        )
        .once_per_batch(),
        triggered(
            when_asked(EventMask::COUNTERS, triggers::nth_counter_put_on_this::<{ CounterKind::Plan as u8 }, 4>),
            effects::sacrifice_this_when_you_do,
            "When the fourth plan counter is put on this enchantment, sacrifice it. When you do, return up to two target instant and/or sorcery cards from your graveyard to your hand.",
        ),
    ],
    delayed: &[delayed_trigger(return_them, "When you do, return up to two target instant and/or sorcery cards from your graveyard to your hand.")
        .targets(&[up_to(2, instant_or_sorcery_card_in_your_graveyard)])],
    ..oracle::REWRITE_HISTORY
};
