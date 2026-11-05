use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of creatures that died this way." (CR 608.2, 120.3, 701.8a).
fn destroy_all_nonblack_creatures_hellfire(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.chars(id).colors.has(q.words(item.object).color(Color::Black)))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_0);
    g.end_sentence();
    let amount_1 = {
        let q: &dyn Query = g;
        (objects_0
            .iter()
            .copied()
            .filter(|&o| {
                q
                    .history()
                    .all()
                    .iter()
                    .any(|r| {
                        matches!(
                            r.event, Event::ZoneChange { old, to, .. } if old == o && to
                            .is(ZoneKind::Graveyard)
                        )
                    })
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| q.chars(id).is(CardType::Creature))
            .collect::<Vec<ObjectId>>()
            .len() as i64 + 3) as i64
    };
    let recipients_1 = Some(item.controller)
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, amount_1 as u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_nonblack_creatures_hellfire),
    ..oracle::HELLFIRE
};
