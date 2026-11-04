use cantrip_api::*;
use cantrip_mechanics::{library, tokens, triggers};
use cantrip_oracle as oracle;

/// "A Scout, Pirate or Rogue" (CR 205.3): the subtypes it has as the layers make it, the three read through the words of
/// Vaan's text (CR 612.2).
fn scout_pirate_or_rogue(chars: &Chars, words: Words) -> bool {
    chars.has_subtype(words.subtype(Subtype::Scout)) || chars.has_subtype(words.subtype(Subtype::Pirate)) || chars.has_subtype(words.subtype(Subtype::Rogue))
}

/// "Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player" (CR 510.2, 603.2c): the
/// damage the batch dealt is one event to the trigger, one trigger for each player damaged (`sums_per_subject`), with "that
/// player" fixed as its subject. Damage a Scout, Pirate or Rogue deals to a planeswalker or a creature does not count.
fn scouts_pirates_or_rogues_deal_combat_damage_to_a_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(p), kind, .. }
            if kind.is_combat() && q.controller(*from) == me.controller && scout_pirate_or_rogue(&q.chars(*from), q.words(me.source)) =>
        {
            Fired::about(Target::Player(*p))
        }
        _ => None,
    }
}

/// "Exile the top card of that player's library. You may cast it. If you don't, create a Treasure token." (CR 608.2g, 601.2):
/// the card is cast now, with the ability on the stack, paying its costs, and is offered only if it could be cast (it is not a
/// land, and has a legal target for all it needs); not cast for any reason - declined, impossible, given up (CR 733.1) or an
/// empty library - the Treasure is made.
fn steal_or_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(whose) = item.subject else { return Ok(()) };
    if !library::exile_top_may_cast(g, whose, item.controller, item.object)? {
        tokens::create(g, item.controller, &tokens::TREASURE, 1)?;
    }
    Ok(())
}

/// "Put a +1/+1 counter on each Scout, Pirate, and Rogue you control" (CR 608.2c): every such permanent its controller controls
/// as this resolves, Vaan among them, all at once (CR 101.4, 603.2c).
fn counter_on_each_scout_pirate_and_rogue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let puts: Vec<(ObjectId, CounterKind, i16)> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| scout_pirate_or_rogue(&g.chars(id), words))
        .map(|id| (id, CounterKind::PlusOne, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, scouts_pirates_or_rogues_deal_combat_damage_to_a_player),
            steal_or_treasure,
            "Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.",
        )
        .sums_per_subject(),
        triggered(
            triggers::YOU_CAST_A_SPELL_YOU_DONT_OWN,
            counter_on_each_scout_pirate_and_rogue,
            "Whenever you cast a spell you don't own, put a +1/+1 counter on each Scout, Pirate, and Rogue you control.",
        ),
    ],
    ..oracle::VAAN_STREET_THIEF
};
