use cantrip_api::*;
use cantrip_mechanics::{half_of_library_rounded_up, sneak, targets};
use cantrip_oracle as oracle;

/// "Target opponent mills half their library, rounded up" (CR 701.17a): half of the library as the spell resolves, so cards drawn or
/// added in response count; nothing if the opponent is no longer a legal target (CR 608.2b).
fn mill_half(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let half = half_of_library_rounded_up(g, opponent);
    g.mill(opponent, half);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], mill_half),
    ways: &[sneak::way(mana("{1}{U}"), "Sneak {1}{U}")],
    ..oracle::KITSUNES_TECHNIQUE
};
