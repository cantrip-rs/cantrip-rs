use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever this creature blocks or becomes blocked by one or more Orcs" (CR 603.2).
fn when_creature_blocks_becomes_blocked_by(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *blocker;
    let other = *attacker;
    let subject = Target::Object(id);
    if asker.source == id
        && q.chars(other).has_subtype(q.words(asker.source).subtype(Subtype::Orc))
    {
        return Fired::about(subject);
    }
    let id = *attacker;
    let other = *blocker;
    let subject = Target::Object(id);
    if asker.source == id
        && q.chars(other).has_subtype(q.words(asker.source).subtype(Subtype::Orc))
    {
        return Fired::about(subject);
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_by),
                effects::pump_self::<0, 2>,
                "Whenever this creature blocks or becomes blocked by one or more Orcs, this creature gets +0/+2 until end of turn.",
            )
            .once_per_batch(),
    ],
    ..oracle::DWARVEN_SOLDIER
};
