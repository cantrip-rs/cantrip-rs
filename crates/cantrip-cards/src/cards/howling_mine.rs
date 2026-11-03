use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The intervening condition is checked at the step and on resolution, using the source's last status if it left (CR 603.4, 608.2h).
fn this_is_untapped(q: &dyn Query, asker: &Asker) -> bool {
    !q.obj(asker.source).tapped
}

/// The player whose draw step began draws one additional card (CR 121.1).
fn that_player_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = item.subject.player() && g.state().in_game(player) {
        g.draw(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::STEP_BEGAN, triggers::each_players_draw_step),
        that_player_draws,
        "At the beginning of each player's draw step, if this artifact is untapped, that player draws an additional card.",
    ).condition(this_is_untapped)],
    ..oracle::HOWLING_MINE
};
