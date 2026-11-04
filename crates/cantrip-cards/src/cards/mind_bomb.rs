use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Each player may discard up to three cards. Mind Bomb deals damage to each player equal to 3 minus the number of cards they discarded this way." (CR 608.2, 120.3, 701.9a).
fn player_discard_up_three_cards(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let players_0: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut each_2: Vec<Vec<ObjectId>> = vec![Vec::new(); players_0.len()];
    let mut all_2: Vec<ObjectId> = Vec::new();
    for (index_0, bound_0) in players_0.iter().copied().enumerate() {
        let selected_1_player = bound_0;
        let selected_1_candidates = {
            let q: &dyn Query = g;
            Some(bound_0)
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .collect::<Vec<ObjectId>>()
        };
        let selected_1 = choose::select(
            g,
            selected_1_player,
            item.object,
            Purpose::Discard,
            selected_1_candidates,
            select::Selection::of(select::Count::UpTo(3), &[]),
        )?;
        each_2[index_0] = selected_1.clone();
        all_2.extend(each_2[index_0].iter().copied());
    }
    g.end_sentence();
    let cards_3 = all_2.clone();
    let discarded_3 = g.discard_together(&cards_3);
    g.end_sentence();
    let players_4: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut gathered_5: Vec<(ObjectId, Target, u32)> = Vec::new();
    for bound_0 in players_4.iter().copied() {
        let amount_6 = {
            let q: &dyn Query = g;
            (3
                - discarded_3
                    .clone()
                    .into_iter()
                    .filter(|&o| q.state().obj(o).owner == bound_0)
                    .collect::<Vec<ObjectId>>()
                    .len() as i64)
                .max(0)
        };
        for t in Some(bound_0).into_iter().map(Target::Player).collect::<Vec<Target>>() {
            gathered_5.push((item.object, t, amount_6 as u32));
        }
    }
    g.damage_together(&gathered_5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], player_discard_up_three_cards),
    ..oracle::MIND_BOMB
};
