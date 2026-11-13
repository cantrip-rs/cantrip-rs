use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less" (CR 603.2).
fn when_enchanted_creature_blocks_becomes_blocked(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let enchanted = q.attached_to(asker.source)?;
    let other = triggers::other_creature_in_block_with_this(q, enchanted, event)?;
    if q.chars(other).toughness.is_some_and(|t| t <= 3) {
        Some(Fired { subject: Target::Object(enchanted), data: [other.0, u32::MAX] })
    } else {
        None
    }
}

/// "At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature." (CR 608.2, 122.1).
fn put_counter_on_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = g.event_objects(item).first().copied() else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way,
/// put a +1/+1 counter on the first creature." (CR 603.7c, 701.8a): the other creature is destroyed, and only if it was is the
/// end-step ability set up about the enchanted creature.
fn destroy_other_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let other: Vec<ObjectId> = g.event_objects(item).first().copied().into_iter().collect();
    let destroyed = g.destroy_all(&other).into_objects();
    g.end_sentence();
    if !destroyed.is_empty() {
        let enchanted: Vec<ObjectId> = item.subject.object().into_iter().collect();
        g.delayed_trigger_about(
            DelayedWhen::Step { step: Step::End, on_turn_of: None },
            item.object,
            item.controller,
            AbilityRef::delayed(item.ability.card, 0),
            &enchanted,
        );
        g.end_sentence();
    }
    Ok(())
}

/// "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature." (CR 608.2).
fn destroy_other_creature_combat_beginning(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let about: Vec<ObjectId> = Some(ObjectId(item.data[0]))
        .filter(|o| *o != ObjectId::NONE)
        .into_iter()
        .collect::<Vec<ObjectId>>();
    let subject = item
        .subject
        .object()
        .into_iter()
        .next()
        .map_or(Target::None, Target::Object);
    g.delayed_trigger_with(
        DelayedWhen::Step {
            step: Step::EndOfCombat,
            on_turn_of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 1),
        subject,
        &about,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::BLOCKS,
                when_enchanted_creature_blocks_becomes_blocked,
            ),
            destroy_other_creature_combat_beginning,
            "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            put_counter_on_enchanted,
            "At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.",
        ),
        delayed_trigger(
            destroy_other_creature,
            "Destroy the other creature at end of combat.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::INFINITE_AUTHORITY
};
