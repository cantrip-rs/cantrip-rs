use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_mechanics::expend::you_expend;
use cantrip_oracle as oracle;

/// "When this enchantment enters and whenever you expend 4" is one ability with two trigger conditions (CR 603.2): this
/// permanent entering, or its controller expending 4 (CR 700.14). One that enters after its controller already spent their
/// fourth mana this turn doesn't see that payment (the expend rulings).
fn enters_or_you_expend_4(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::this_enters(q, me.source, event).or_else(|| you_expend::<4>(q, me, event))
}

/// "Put a stash counter on it." (CR 122.1): on this enchantment, nothing once it has left the battlefield (CR 400.7).
fn stash_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Stash, 1);
    Ok(())
}

/// "Discard your hand, then draw cards equal to the number of stash counters on this enchantment." (CR 701.9, 121.1): the
/// counters it had as it was sacrificed to pay the cost, read as it last existed (CR 608.2h); the draw happens whatever the
/// hand was.
fn discard_hand_then_draw_for_stash(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let stash = g.obj(item.object).counters(CounterKind::Stash) as u32;
    effects::discard_hand_then_draw(g, item, stash)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::SPELL_CAST.0), enters_or_you_expend_4),
            stash_counter,
            "When this enchantment enters and whenever you expend 4, put a stash counter on it. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
        ),
        ability(
            Cost::mana(mana("{1}{R}")).and(SACRIFICE_THIS),
            discard_hand_then_draw_for_stash,
            "{1}{R}, Sacrifice this enchantment: Discard your hand, then draw cards equal to the number of stash counters on this enchantment.",
        ),
    ],
    ..oracle::HOARDERS_OVERFLOW
};
