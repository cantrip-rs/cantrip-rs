use cantrip_api::*;
use cantrip_mechanics::{targets, vivid};
use cantrip_oracle as oracle;

/// "Vivid - This spell costs {1} less to cast for each color among permanents you control" (CR 601.2f, 700.4): judged once, as the
/// total cost is determined, for the player casting it; generic mana only (CR 118.7a), and colorless is no color.
fn cheaper_per_color(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(vivid::count(q, caster))
}

/// "Tap up to two target creatures. Put a stun counter on each of them. Draw a card." The creatures still legal as it resolves are
/// tapped (CR 701.26a, 608.2b), already tapped or not, and each gets its counter (CR 122.1d), all at once; with every target
/// illegal the spell does not resolve and nothing is drawn (the rulings), while with none chosen it resolves and draws.
fn tap_stun_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    for &creature in &creatures {
        g.tap(creature);
    }
    let puts: Vec<(ObjectId, CounterKind, i16)> = creatures.into_iter().map(|creature| (creature, CounterKind::Stun, 1)).collect();
    g.add_counters_together(&puts);
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cheaper_per_color],
    spell: spell(&[targets::CREATURE.between(0, 2)], tap_stun_and_draw),
    ..oracle::RIME_CHILL
};
