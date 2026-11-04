use cantrip_api::*;
use cantrip_mechanics::{fight, flashback, targets};
use cantrip_oracle as oracle;

/// "Choose target creature you control. It deals damage equal to its power to each other creature. If this spell was cast from a
/// graveyard, discard your hand and draw four cards." The creature is the source of the damage, not the spell (the ruling), so
/// its abilities count; its power is the one it has as the spell resolves (CR 608.2h), and every other creature on the battlefield,
/// whoever controls it, is dealt that much at once (CR 120.3); a power of 0 or less deals none (CR 120.8). If the target is illegal
/// as the spell resolves, none of it happens, the discard and draw included (CR 608.2b). The zone it was cast from is the spell's
/// own (CR 601.2a), by flashback or any other permission; a copy was never cast (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = fight::damage_equal_to_power(g, creature);
    let hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != creature && g.chars(id).is(CardType::Creature))
        .map(|id| (creature, Target::Object(id), power))
        .collect();
    g.damage_together(&hits);
    if g.obj(item.object).cast_from == Some(ZoneKind::Graveyard) {
        g.discard_hand(item.controller);
        g.draw(item.controller, 4);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], effect),
    ways: &[flashback::way(cost("{5}{R}{R}"), "Flashback {5}{R}{R}")],
    ..oracle::NIBELHEIM_AFLAME
};
