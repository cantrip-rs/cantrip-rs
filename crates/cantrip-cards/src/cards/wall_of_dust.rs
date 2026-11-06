use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever this creature blocks a creature" (CR 603.2).
fn when_creature_blocks_creature(
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

/// "Whenever this creature blocks a creature, that creature can't attack during its controller's next turn." (CR 608.2, 611.2a).
fn creature_cant_attack_during_controllers(
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
                    EffectKind::CantAttack,
                    Duration::DURING_ITS_CONTROLLERS_NEXT_TURN,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_creature),
            creature_cant_attack_during_controllers,
            "Whenever this creature blocks a creature, that creature can't attack during its controller's next turn.",
        ),
    ],
    ..oracle::WALL_OF_DUST
};
