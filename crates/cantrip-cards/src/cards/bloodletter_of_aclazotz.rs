use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If an opponent would lose life during your turn" (CR 614.1a): a loss of life by an opponent of Bloodletter's
/// controller, whatever causes it - an effect, a cost paid in life (CR 119.4) or damage (CR 120.3a) - while its controller
/// is the active player. Only the loss is replaced: damage dealt stays as it is, and so does the life lifelink gains for
/// it (the 2023-11-10 ruling).
fn opponent_loses_life_during_your_turn(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::LifeChange(l) = proposal else { return false };
    let state = q.state();
    l.negative && l.magnitude > 0 && state.turn.active == asker.controller && state.opponents(asker.controller).any(|p| p == l.player)
}

/// "... they lose twice that much life instead."
fn twice_that_much(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.magnitude = l.magnitude.saturating_mul(2);
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_life_during_your_turn, twice_that_much, "you lose twice that much life")],
    ..oracle::BLOODLETTER_OF_ACLAZOTZ
};
