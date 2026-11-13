use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap all lands target player controls and that player loses all unspent mana." (CR 608.2, 701.26a).
fn tap_all_lands_target_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Land)
                    && t_0.get(0).copied().and_then(|t: Target| t.player())
                        == Some(q.controller(id))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.tap_together(&objects_0);
    g.end_sentence();
    let Some(player) = t_0.get(0).copied().and_then(|t: Target| t.player()) else { return Ok(()) };
    g.empty_mana_pool(player);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], tap_all_lands_target_player),
    ..oracle::MANA_SHORT
};
