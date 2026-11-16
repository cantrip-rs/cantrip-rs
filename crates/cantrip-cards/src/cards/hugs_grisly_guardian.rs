use cantrip_api::*;
use cantrip_mechanics::library::exile_top_play_until_end_of_next_turn;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When Hugs enters, exile the top X cards of your library. Until the end of your next turn, you may play those cards."
/// X is the one Hugs was cast with (CR 107.3m), which the permanent keeps and which is still read if Hugs has left the
/// battlefield by the time the ability resolves.
fn exile_top_x_to_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.obj(item.object).x as usize;
    exile_top_play_until_end_of_next_turn(g, item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_top_x_to_play, "When Hugs enters, exile the top X cards of your library. Until the end of your next turn, you may play those cards.")],
    // "You may play an additional land on each of your turns." (CR 305.2: one more land play for its controller.)
    play_rules: &[PlayRule::ExtraLandPlays(1)],
    ..oracle::HUGS_GRISLY_GUARDIAN
};
