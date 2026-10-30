use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers, zone_cast_from};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell from your hand" (CR 601.2a, 603.2): about the spell, which is still on the
/// stack as the trigger is put there.
fn you_cast_an_instant_or_sorcery_from_your_hand(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match triggers::you_cast_instant_or_sorcery(q, me, event) {
        Some(fired) if matches!(event, Event::SpellCast { spell, .. } if zone_cast_from(q, *spell) == Some(ZoneKind::Hand)) => Some(fired),
        _ => None,
    }
}

/// "Exile that card with a dream counter on it instead of putting it into your graveyard as it resolves": the spell, if it is
/// still on the stack, will be exiled in place of its move to the graveyard, the counter put on it as it arrives (CR 614.1a,
/// 122.2). A spell that is countered does not resolve and goes to the graveyard (CR 701.6a).
fn dream(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        g.exile_spell_instead(spell, ExileInstead::Dream);
    }
    Ok(())
}

/// "Whenever this creature attacks, you may cast a spell from among cards you own in exile with dream counters on them without
/// paying its mana cost." The cast is part of the resolution, ignoring timing (CR 608.2g); an {X} in the cost is 0 (CR 107.3b).
/// Nothing is asked when no such card can be cast; one that cannot be completed is given up (CR 733.1).
fn cast_a_dreamed_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let dreamed: Vec<ObjectId> = g.state().zone(Zone::exile(you)).iter().copied().filter(|&card| g.obj(card).counters(CounterKind::Dream) > 0).collect();
    let castable: Vec<ObjectId> = dreamed.into_iter().filter(|&card| !g.cast_ways_during_resolution(card).is_empty()).collect();
    let chosen = choose::select(g, you, item.object, Purpose::Cast, castable, select::cards(select::up_to(1)))?;
    if let Some(&chosen) = chosen.first() {
        let ways = g.cast_ways_during_resolution(chosen);
        let way = choose::cast_way(g, you, chosen, ways)?.map(|w| w.way);
        g.cast_during_resolution_as(chosen, CastTerms { instead: Some(ManaCost::ZERO), way, ..CastTerms::default() })?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_an_instant_or_sorcery_from_your_hand),
            dream,
            "Whenever you cast an instant or sorcery spell from your hand, exile that card with a dream counter on it instead of putting it into your graveyard as it resolves.",
        ),
        triggered(
            triggers::THIS_ATTACKS,
            cast_a_dreamed_spell,
            "Whenever this creature attacks, you may cast a spell from among cards you own in exile with dream counters on them without paying its mana cost.",
        ),
    ],
    ..oracle::GOLIATH_DAYDREAMER
};
