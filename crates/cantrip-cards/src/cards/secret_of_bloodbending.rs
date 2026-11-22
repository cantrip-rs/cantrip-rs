use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "You control target opponent during their next combat phase. If this spell's additional cost was paid, you control that
/// player during their next turn instead. Exile Secret of Bloodbending." (CR 723.1) The additional cost is the card's
/// waterbend `kicker` (CR 701.67): paid, `item.kicked` is 1. The spell with no legal target does not resolve (CR 608.2b),
/// so neither is exiled.
fn control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.target(0).player() else { return Ok(()) };
    let window = if item.kicked > 0 { ControlWindow::Turn { then_extra_turn: false } } else { ControlWindow::CombatPhase };
    g.control_next(item, player, window);
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], control),
    // "As an additional cost to cast this spell, you may waterbend {10}."
    kicker: waterbend_kicker(mana("{10}")),
    ..oracle::SECRET_OF_BLOODBENDING
};
