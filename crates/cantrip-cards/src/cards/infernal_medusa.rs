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

/// "Whenever this creature blocks a creature, destroy that creature at end of combat. Whenever this creature becomes blocked by a non-Wall creature, destroy that creature at end of combat." (CR 608.2, 701.8a).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        about(q, item, 0)
    };
    g.destroy_all(&objects_2);
    Ok(())
}

/// "Whenever this creature blocks a creature, destroy that creature at end of combat." (CR 608.2).
fn destroy_creature_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    {
        let about: Vec<ObjectId> = Some(ObjectId(item.data[0]))
            .filter(|o| *o != ObjectId::NONE)
            .into_iter()
            .collect::<Vec<ObjectId>>();
        {
            g.delayed_trigger_about(
                DelayedWhen::Step {
                    step: Step::EndOfCombat,
                    on_turn_of: None,
                },
                item.object,
                item.controller,
                AbilityRef::delayed(item.ability.card, 0),
                &about,
            );
        }
    }
    Ok(())
}

/// "Whenever this creature becomes blocked by a non-Wall creature" (CR 603.2).
fn when_creature_becomes_blocked_by_non(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *attacker;
    let other = *blocker;
    let subject = Target::Object(id);
    let data = [other.0, u32::MAX];
    if asker.source == id
        && (!(q.chars(other).has_subtype(q.words(asker.source).subtype(Subtype::Wall)))
            && q.chars(other).is(CardType::Creature))
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever this creature becomes blocked by a non-Wall creature, destroy that creature at end of combat." (CR 608.2).
fn destroy_creature_combat_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    {
        let about: Vec<ObjectId> = Some(ObjectId(item.data[0]))
            .filter(|o| *o != ObjectId::NONE)
            .into_iter()
            .collect::<Vec<ObjectId>>();
        {
            g.delayed_trigger_about(
                DelayedWhen::Step {
                    step: Step::EndOfCombat,
                    on_turn_of: None,
                },
                item.object,
                item.controller,
                AbilityRef::delayed(item.ability.card, 0),
                &about,
            );
        }
    }
    Ok(())
}

/// "Whenever this creature blocks a creature, destroy that creature at end of combat." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    q.event_objects(item)
        .split(|&id| id == ObjectId::NONE)
        .nth(part)
        .map_or_else(Vec::new, <[ObjectId]>::to_vec)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_creature),
            destroy_creature_combat,
            "Whenever this creature blocks a creature, destroy that creature at end of combat.",
        ),
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_becomes_blocked_by_non),
            destroy_creature_combat_2,
            "Whenever this creature becomes blocked by a non-Wall creature, destroy that creature at end of combat.",
        ),
    ],
    delayed: &[
        delayed_trigger(delayed_1, "Destroy that creature."),
    ],
    ..oracle::INFERNAL_MEDUSA
};
