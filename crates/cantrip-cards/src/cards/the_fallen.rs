use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, this creature deals 1 damage to each opponent and planeswalker it has dealt damage to this game." (CR 608.2, 120.3).
fn creature_deals_1_damage_opponent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_0 = {
        let q: &dyn Query = g;
        {
            let mut all: Vec<Target> = Vec::new();
            for t in ({
                let mut all = q
                    .state()
                    .opponents(item.controller)
                    .collect::<Vec<PlayerId>>();
                let other = q
                    .state()
                    .apnap()
                    .filter(|&bound_0: &PlayerId| {
                        (q
                            .history()
                            .all()
                            .iter()
                            .filter_map(|r: &Record| -> Option<Fired> {
                                let event = &r.event;
                                if let Event::Damage { source, target, .. } = event {
                                    let source = *source;
                                    let subject = *target;
                                    if item.object == source
                                        && matches!(* target, Target::Player(p) if bound_0 == p)
                                    {
                                        return Fired::about(subject);
                                    }
                                }
                                None
                            })
                            .count() as i64) >= 1
                    })
                    .collect::<Vec<PlayerId>>();
                all.retain(|p: &PlayerId| other.contains(p));
                all
            })
                .into_iter()
                .map(Target::Player)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            for t in q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Planeswalker)
                        && (q
                            .history()
                            .all()
                            .iter()
                            .filter_map(|r: &Record| -> Option<Fired> {
                                let event = &r.event;
                                if let Event::Damage { source, target, .. } = event {
                                    let source = *source;
                                    let subject = *target;
                                    if item.object == source
                                        && matches!(* target, Target::Object(o) if id == o)
                                    {
                                        return Fired::about(subject);
                                    }
                                }
                                None
                            })
                            .count() as i64) >= 1
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .map(Target::Object)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            all
        }
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            creature_deals_1_damage_opponent,
            "At the beginning of your upkeep, this creature deals 1 damage to each opponent and planeswalker it has dealt damage to this game.",
        ),
    ],
    ..oracle::THE_FALLEN
};
