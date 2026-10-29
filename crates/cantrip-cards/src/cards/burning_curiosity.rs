use cantrip_api::*;
use cantrip_mechanics::blight;
use cantrip_mechanics::library::exile_top_play_until_end_of_next_turn;
use cantrip_oracle as oracle;

/// "Exile the top two cards of your library. If this spell's additional cost was paid, exile the top three cards instead. Until the
/// end of your next turn, you may play those cards." A copy of the spell has the cost paid when the original did (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_top_play_until_end_of_next_turn(g, item.controller, if blight::blighted(item) { 3 } else { 2 });
    Ok(())
}

/// "As an additional cost to cast this spell, you may blight 1" (CR 701.68a, 601.2b): an optional cost, as Cinder Strike's is.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    optional_cost: Some(blight::optional(1)),
    labels: &["Cast without blighting", "Blight 1: put a -1/-1 counter on a creature you control"],
    ..oracle::BURNING_CURIOSITY
};
