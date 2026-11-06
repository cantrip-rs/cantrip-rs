use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target player draws cards equal to half the number of cards in their library and loses half their life. Round up each time." (CR 608.2, 121.1, 119.3).
fn target_player_draws_cards_equal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let amount_0 = {
        let q: &dyn Query = g;
        (t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.player())
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
            .len() as i64)
            .saturating_add(1)
            .div_euclid(2) as u32
    };
    let players_0 = targets::player(g, item, 0);
    if let Some(player) = players_0 {
        g.draw(player, amount_0);
    }
    g.end_sentence();
    let amount_1 = {
        let q: &dyn Query = g;
        t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.player())
            .map_or(0, |p: PlayerId| i64::from(q.state().player(p).life))
            .saturating_add(1)
            .div_euclid(2)
            .max(0) as i32
    };
    if let Some(player) = players_0 {
        g.lose_life(player,(amount_1) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::PLAYER],
        target_player_draws_cards_equal,
    ),
    ..oracle::PEER_INTO_THE_ABYSS
};
