use cantrip_api::*;
use cantrip_mechanics::library::exile_top_play_this_turn;
use cantrip_oracle as oracle;

/// "Exile the top X cards of your library, where X is Jessica Jones's power. You may play those cards this turn." (CR 406.3,
/// 305.1, 601.2, 514.2): X is her power as the ability resolves, as she last existed if she has left the battlefield (CR 113.7a,
/// 608.2h) and 0 for a negative power; the permission is to play, so a land among the cards may be the turn's land drop. A stun
/// counter is no part of the exiling and changes no power.
fn exile_top_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.chars(item.object).power.unwrap_or(0).max(0) as usize;
    exile_top_play_this_turn(g, item.controller, x);
    Ok(())
}

/// "{T}, Put a stun counter on Jessica Jones" (CR 118.3, 122.1d) is a cost: it is put on as the ability is activated.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}").and(put_counters(CounterKind::Stun, 1)),
        exile_top_cards,
        "{T}, Put a stun counter on Jessica Jones: Exile the top X cards of your library, where X is Jessica Jones's power. You may play those cards this turn. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )],
    ..oracle::JESSICA_JONES_PRIVATE_EYE
};
