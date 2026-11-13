use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "After this main phase, there are two additional combat phases" (CR 500.8, 505.1): the phases follow the main phase under way,
/// one after the other with no main phase between them; cast outside a main phase they are not added at all (the ruling), and
/// the second sentence still applies. "At the beginning of each combat this turn" is a delayed triggered ability (CR 603.7b)
/// made as the spell resolves, so it sees the added combats and the turn's own combat alike, if still ahead.
fn two_more_combats(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.state().turn.step().is_main() {
        g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Whole(Phase::Combat), PhaseSpec::Whole(Phase::Combat)]);
    }
    g.end_sentence();
    let this = g.card_id(&CARD);
    g.delayed_trigger(DelayedWhen::ThisTurn, item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

/// "Untap all creatures that attacked this turn" (CR 701.26b, 508.1, 400.7): creatures of either player, the same objects that
/// were declared attacking.
fn untap(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    effects::untap_creatures_that_attacked_this_turn(g);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], two_more_combats),
    delayed: &[triggered(
        triggers::EACH_PLAYERS_BEGINNING_OF_COMBAT,
        untap,
        "At the beginning of each combat this turn, untap all creatures that attacked this turn.",
    )],
    ..oracle::FULL_THROTTLE
};
