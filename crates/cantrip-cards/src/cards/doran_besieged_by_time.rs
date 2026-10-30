use cantrip_api::*;
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// "Each creature spell you cast with toughness greater than its power costs {1} less to cast" (CR 601.2f, 118.7c): asked of
/// the spell's own characteristics as it is cast, so only your own spells, and a generic reduction takes only the
/// generic part of the total cost.
fn tough_creature_spells_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) {
        return CostDelta::NONE;
    }
    let chars = q.chars(spell);
    if chars.is(CardType::Creature) && chars.toughness.unwrap_or(0) > chars.power.unwrap_or(0) { CostDelta::less(1) } else { CostDelta::NONE }
}

/// "Whenever a creature you control attacks or blocks" (CR 508.1m, 509.1, 603.2): each such creature, separately, the
/// ability being about it (`Fired::about`). A blocker triggers once however many attackers it blocks (CR 509.3a): the
/// declared block alone.
fn creature_you_control_attacks_or_blocks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker: creature, .. } | Event::Blocks { blocker: creature, declared: true, .. }
            if q.controller(*creature) == me.controller =>
        {
            Fired::about(Target::Object(*creature))
        }
        _ => None,
    }
}

/// "It gets +X/+X until end of turn, where X is the difference between its power and toughness" (CR 611.2a, 107.1b): X is
/// worked out once, as the ability resolves, from the creature's power and toughness then - the larger less the smaller,
/// whichever way round - and a creature that has left the battlefield gets nothing.
fn difference_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let chars = g.chars(creature);
    let (power, toughness) = (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0));
    let x = (i32::from(power) - i32::from(toughness)).abs() as i16;
    pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[tough_creature_spells_cost_less],
    abilities: &[triggered(
        when_asked(EventMask(EventMask::ATTACKS.0 | EventMask::BLOCKS.0), creature_you_control_attacks_or_blocks),
        difference_pump,
        "Whenever a creature you control attacks or blocks, it gets +X/+X until end of turn, where X is the difference between its power and toughness.",
    )],
    ..oracle::DORAN_BESIEGED_BY_TIME
};
