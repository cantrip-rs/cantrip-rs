use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target player draws two cards. Tap up to two target creatures. Put a stun counter on each of them." (CR 608.2c): the
/// player draws if still a legal target, whatever has become of the creatures, and the creatures that are legal targets are
/// tapped together, then each gets a stun counter together (CR 701.26a, 122.1d), tapped already or not.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    g.end_sentence();
    let creatures = targets::legal_objects(g, item);
    for &id in &creatures {
        g.tap(id);
    }
    let puts: Vec<(ObjectId, CounterKind, i16)> = creatures.into_iter().map(|id| (id, CounterKind::Stun, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, targets::CREATURE.between(0, 2)], effect),
    ..oracle::HOMESICKNESS
};
