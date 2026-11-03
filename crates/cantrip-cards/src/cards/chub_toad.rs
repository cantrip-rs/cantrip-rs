use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever this creature blocks or becomes blocked" (CR 603.2).
fn when_creature_blocks_becomes_blocked(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, declared: true, .. } = event else { return None };
    let id = *blocker;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature blocks or becomes blocked" (CR 603.2).
fn when_creature_blocks_becomes_blocked_2(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    let data = [u32::MAX, id.0];
    if asker.source == id {
        return Some(Fired { subject, data });
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked),
            effects::pump_self::<2, 2>,
            "Whenever this creature blocks or becomes blocked, it gets +2/+2 until end of turn.",
        ),
        triggered(
                when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_2),
                effects::pump_self::<2, 2>,
                "Whenever this creature blocks or becomes blocked, it gets +2/+2 until end of turn.",
            )
            .once_per_batch()
            .batch_by_data(),
    ],
    ..oracle::CHUB_TOAD
};
