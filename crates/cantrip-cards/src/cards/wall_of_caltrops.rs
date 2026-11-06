use cantrip_api::*;
use cantrip_mechanics::effects;
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
    if asker.source == id && q.chars(other).is(CardType::Creature)
        && ((q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(asker.source == id)
                    && q
                        .chars(id)
                        .has_subtype(q.words(asker.source).subtype(Subtype::Wall))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| {
                let attacker = Some(ObjectId(data[0])).filter(|o| *o != ObjectId::NONE);
                q.state()
                    .blocks
                    .iter()
                    .any(|b| b.blocker == id && Some(b.attacker) == attacker)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 1
            && (q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    !(q
                        .chars(id)
                        .has_subtype(q.words(asker.source).subtype(Subtype::Wall)))
                        && q.chars(id).is(CardType::Creature)
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    let attacker = Some(ObjectId(data[0]))
                        .filter(|o| *o != ObjectId::NONE);
                    q.state()
                        .blocks
                        .iter()
                        .any(|b| b.blocker == id && Some(b.attacker) == attacker)
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64) == 0)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever this creature blocks a creature, if at least one other Wall creature is blocking that creature and no non-Wall creatures are blocking that creature, this creature gains banding until end of turn." (CR 608.2, 611.2a).
fn if_least_one_other_wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !{
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(item.object == id)
                    && q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Wall))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| {
                let attacker = Some(ObjectId(item.data[0]))
                    .filter(|o| *o != ObjectId::NONE);
                q.state()
                    .blocks
                    .iter()
                    .any(|b| b.blocker == id && Some(b.attacker) == attacker)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 1
            && (q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    !(q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Wall)))
                        && q.chars(id).is(CardType::Creature)
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    let attacker = Some(ObjectId(item.data[0]))
                        .filter(|o| *o != ObjectId::NONE);
                    q.state()
                        .blocks
                        .iter()
                        .any(|b| b.blocker == id && Some(b.attacker) == attacker)
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64) == 0
    } {
        return Ok(());
    }
    instruction(g, |g| effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Banding));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_creature),
            if_least_one_other_wall,
            "Whenever this creature blocks a creature, if at least one other Wall creature is blocking that creature and no non-Wall creatures are blocking that creature, this creature gains banding until end of turn.",
        ),
    ],
    ..oracle::WALL_OF_CALTROPS
};
