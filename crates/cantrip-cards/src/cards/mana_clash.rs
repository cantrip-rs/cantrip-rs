use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "You and target opponent each flip a coin. Mana Clash deals 1 damage to each player whose coin comes up tails. Repeat this process until both players' coins come up heads on the same flip." (CR 608.2, 120.3).
fn target_opponent_flip_coin_mana(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    loop {
        let mut again_0 = false;
        let players_1: Vec<PlayerId> = {
            let mut all: Vec<PlayerId> = Vec::new();
            if !all.contains(&item.controller) {
                all.push(item.controller);
            }
            for p in t_0
                .get(0)
                .copied()
                .and_then(|t: Target| t.player())
                .into_iter()
                .collect::<Vec<PlayerId>>()
            {
                if !all.contains(&p) {
                    all.push(p);
                }
            }
            all
        };
        let mut each_3 = vec![Default::default(); players_1.len()];
        for (index_1, bound_0) in players_1.iter().copied().enumerate() {
            let player_2 = bound_0;
            let flip_2 = g.flip_coin_for_heads(player_2);
            each_3[index_1] = flip_2;
        }
        g.end_sentence();
        let recipients_4 = {
            let q: &dyn Query = g;
            q.state()
                .apnap()
                .filter(|&bound_0: &PlayerId| {
                    players_1
                        .iter()
                        .zip(&each_3)
                        .any(|(&flipper, &heads)| flipper == bound_0 && heads == false)
                })
                .collect::<Vec<PlayerId>>()
                .into_iter()
                .map(Target::Player)
                .collect::<Vec<Target>>()
        };
        let batch: Vec<(ObjectId, Target, u32)> = recipients_4
            .iter()
            .map(|t| (item.object, *t, 1))
            .collect();
        g.damage_together(&batch);
        g.end_sentence();
        if !(((each_3.iter().filter(|&&flip| !flip).count() as i64)) == 0) {
            again_0 = true;
            g.end_sentence();
        }
        g.end_sentence();
        if !again_0 {
            break;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::OPPONENT],
        target_opponent_flip_coin_mana,
    ),
    ..oracle::MANA_CLASH
};
