use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Each player loses a third of their life, then discards a third of the cards in their hand, then sacrifices a third of the creatures they control of their choice, then sacrifices a third of the lands they control of their choice. Round up each time." (CR 608.2, 701.21a, 701.9a).
fn player_loses_third_their_life(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let players_0: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let losses: Vec<_> = players_0.iter().map(|&player| {
        let life = i64::from(g.state().player(player).life).max(0);
        (player, ((life + 2) / 3) as i32)
    }).collect();
    g.lose_life_together(&losses);
    g.end_sentence();
    let players_2: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut gathered_3: Vec<ObjectId> = Vec::new();
    for bound_0 in players_2.iter().copied() {
        let cards: Vec<_> = g.state().zone(Zone::hand(bound_0)).iter().copied()
            .filter(|&id| !g.obj(id).token && !g.obj(id).card_copy)
            .collect();
        let count = (cards.len() as u32).div_ceil(3);
        gathered_3.extend(choose::select(g, bound_0, item.object, Purpose::Discard,
            cards, select::cards(select::exactly(count)))?);
    }
    let cards_5 = gathered_3.clone();
    g.discard_together(&cards_5);
    g.end_sentence();
    let players_6: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut gathered_7: Vec<(PlayerId, ObjectId)> = Vec::new();
    for bound_0 in players_6.iter().copied() {
        let objects_8 = {
            let q: &dyn Query = g;
            q.state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| q.chars(id).is(CardType::Creature))
                .collect::<Vec<ObjectId>>()
        };
        let players_8 = Some(bound_0).into_iter().collect::<Vec<PlayerId>>();
        let mut picks_8: Vec<(PlayerId, ObjectId)> = Vec::new();
        let count_8 = {
            let q: &dyn Query = g;
            select::Count::AsManyAsCan(
                (q
                    .state()
                    .battlefield()
                    .iter()
                    .copied()
                    .filter(|&id| {
                        q.chars(id).is(CardType::Creature) && bound_0 == q.controller(id)
                    })
                    .collect::<Vec<ObjectId>>()
                    .len() as i64)
                    .saturating_add(2)
                    .div_euclid(3) as u32,
            )
        };
        for player in players_8 {
            let eligible: Vec<ObjectId> = objects_8
                .iter()
                .copied()
                .filter(|&o| g.controller(o) == player && g.can_sacrifice(player, o))
                .collect();
            let taken = match count_8.clone() {
                select::Count::AsManyAsCan(n) if eligible.len() <= n as usize => eligible,
                _ => {
                    choose::select(
                        g,
                        player,
                        item.object,
                        Purpose::Sacrifice,
                        eligible,
                        select::Selection::of(count_8.clone(), &[]),
                    )?
                }
            };
            for object in taken {
                picks_8.push((player, object));
            }
        }
        gathered_7.extend(picks_8.iter().copied());
    }
    g.sacrifice_each(&gathered_7);
    g.end_sentence();
    let players_9: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut gathered_10: Vec<(PlayerId, ObjectId)> = Vec::new();
    for bound_0 in players_9.iter().copied() {
        let objects_11 = {
            let q: &dyn Query = g;
            q.state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| q.chars(id).is(CardType::Land))
                .collect::<Vec<ObjectId>>()
        };
        let players_11 = Some(bound_0).into_iter().collect::<Vec<PlayerId>>();
        let mut picks_11: Vec<(PlayerId, ObjectId)> = Vec::new();
        let count_11 = {
            let q: &dyn Query = g;
            select::Count::AsManyAsCan(
                (q
                    .state()
                    .battlefield()
                    .iter()
                    .copied()
                    .filter(|&id| {
                        q.chars(id).is(CardType::Land) && bound_0 == q.controller(id)
                    })
                    .collect::<Vec<ObjectId>>()
                    .len() as i64)
                    .saturating_add(2)
                    .div_euclid(3) as u32,
            )
        };
        for player in players_11 {
            let eligible: Vec<ObjectId> = objects_11
                .iter()
                .copied()
                .filter(|&o| g.controller(o) == player && g.can_sacrifice(player, o))
                .collect();
            let taken = match count_11.clone() {
                select::Count::AsManyAsCan(n) if eligible.len() <= n as usize => eligible,
                _ => {
                    choose::select(
                        g,
                        player,
                        item.object,
                        Purpose::Sacrifice,
                        eligible,
                        select::Selection::of(count_11.clone(), &[]),
                    )?
                }
            };
            for object in taken {
                picks_11.push((player, object));
            }
        }
        gathered_10.extend(picks_11.iter().copied());
    }
    g.sacrifice_each(&gathered_10);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], player_loses_third_their_life),
    ..oracle::POX
};
