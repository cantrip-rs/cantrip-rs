use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever this creature blocks or becomes blocked by a creature" (CR 603.2).
fn when_creature_blocks_becomes_blocked_by(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *blocker;
    let other = *attacker;
    let subject = Target::Object(id);
    let data = [other.0, u32::MAX];
    if asker.source == id && q.chars(other).is(CardType::Creature) {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn." (CR 608.2, 611.2a).
fn creature_cant_be_regenerated(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = Some(ObjectId(item.data[0]))
        .filter(|o| *o != ObjectId::NONE)
        .into_iter()
        .collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::CantBeRegenerated,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// "Whenever this creature blocks or becomes blocked by a creature" (CR 603.2).
fn when_creature_blocks_becomes_blocked_by_2(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *attacker;
    let other = *blocker;
    let subject = Target::Object(id);
    let data = [other.0, u32::MAX];
    if asker.source == id && q.chars(other).is(CardType::Creature) {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn." (CR 608.2, 611.2a).
fn creature_cant_be_regenerated_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = Some(ObjectId(item.data[0]))
        .filter(|o| *o != ObjectId::NONE)
        .into_iter()
        .collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::CantBeRegenerated,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_by),
            creature_cant_be_regenerated,
            "Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn.",
        ),
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_by_2),
            creature_cant_be_regenerated_2,
            "Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn.",
        ),
    ],
    ..oracle::LIM_D_LS_COHORT
};
