use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This spell can't be countered. Target opponent reveals their hand. Exile all noncreature, nonland cards from that player's hand and graveyard." (CR 115.1).
fn spell_cant_be_countered_target_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Player(p) if q.state().opponents(asker.controller).collect:: < Vec <
        PlayerId > > ().contains(& p)
    )
}

/// "This spell can't be countered. Target opponent reveals their hand. Exile all noncreature, nonland cards from that player's hand and graveyard." (CR 608.2, 406.3, 701.20a).
fn spell_cant_be_countered_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.player())
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    let mut owners_0: Vec<PlayerId> = objects_0
        .iter()
        .map(|o| g.state().obj(*o).owner)
        .collect();
    owners_0.dedup();
    for owner in owners_0 {
        let cards: Vec<ObjectId> = objects_0
            .iter()
            .copied()
            .filter(|o| g.state().obj(*o).owner == owner)
            .collect();
        g.reveal(owner, &cards);
    }
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        {
            let mut all: Vec<ObjectId> = Vec::new();
            for id in q
                .state()
                .apnap()
                .collect::<Vec<PlayerId>>()
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    !(q.chars(id).is(CardType::Creature))
                        && !(q.chars(id).is(CardType::Land))
                        && (q.state().obj(id).zone.kind() == ZoneKind::Hand
                            && t_0.get(0).copied().and_then(|t: Target| t.player())
                                == Some(q.state().obj(id).owner))
                })
                .collect::<Vec<ObjectId>>()
            {
                if !all.contains(&id) {
                    all.push(id);
                }
            }
            for id in q
                .state()
                .apnap()
                .collect::<Vec<PlayerId>>()
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::graveyard(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    !(q.chars(id).is(CardType::Creature))
                        && !(q.chars(id).is(CardType::Land))
                        && (q.state().obj(id).zone.kind() == ZoneKind::Graveyard
                            && t_0.get(0).copied().and_then(|t: Target| t.player())
                                == Some(q.state().obj(id).owner))
                })
                .collect::<Vec<ObjectId>>()
            {
                if !all.contains(&id) {
                    all.push(id);
                }
            }
            all
        }
    };
    g.exile_all(&objects_1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(spell_cant_be_countered_target_target_0)],
        spell_cant_be_countered_target,
    ),
    ..oracle::THOUGHT_DISTORTION
};
