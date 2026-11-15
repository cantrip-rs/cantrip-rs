use cantrip_api::*;
use cantrip_mechanics::{flashback, history, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 blue and red Otter creature token with prowess. If this spell was cast from anywhere other than your hand,
/// put a +1/+1 counter on that creature." Where it was cast from is the cast record's zone (CR 601.2a): its caster's own hand
/// is "your hand", every other zone (a graveyard by flashback, or a card of another player's hand) is "anywhere other". A copy
/// was never cast (CR 707.10), so it has no cast record and gets no counter.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let otters = tokens::create(g, item.controller, &tokens::OTTER, 1)?;
    g.end_sentence();
    let owner = g.obj(item.object).owner;
    if history::cast_of(g, item.object).is_some_and(|cast| !(cast.from == ZoneKind::Hand && cast.player == owner)) {
        for otter in otters {
            g.add_counters(otter, CounterKind::PlusOne, 1);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{3}{U}"), "Flashback {3}{U}")],
    ..oracle::OTTERBALL_ANTICS
};

pub static EXTRA: &[&CardDef] = &[&tokens::OTTER];
