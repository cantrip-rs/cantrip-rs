use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// A creature dealt damage by this source this turn dies (CR 603.2, 603.10a).
fn when_creature_dealt_damage_by_axelrod(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard)
        && q.chars(*old).is(CardType::Creature)
        && q.history().this_turn().iter().any(|r| matches!(r.event,
            Event::Damage { source, target: Target::Object(target), amount, .. }
                if source == asker.source && target == *old && amount > 0))
    {
        return Fired::about(Target::Object(*old));
    }
    None
}

/// Gain 1 life, then deal 1 damage (CR 608.2c, 119.3, 120.3).
fn gain_life_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    g.damage(item.object, item.target(0), 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_creature_dealt_damage_by_axelrod),
            gain_life_and_damage,
            "Whenever a creature dealt damage by Axelrod Gunnarson this turn dies, you gain 1 life and Axelrod Gunnarson deals 1 damage to target player or planeswalker.",
        ).targets(&[targets::PLAYER_OR_PLANESWALKER]),
    ],
    ..oracle::AXELROD_GUNNARSON
};
