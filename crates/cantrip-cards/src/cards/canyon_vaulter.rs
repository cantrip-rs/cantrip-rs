use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_oracle as oracle;

/// "Whenever this creature saddles a Mount or crews a Vehicle during your main phase" (CR 702.122b, 702.171c, 603.2, 505.1): the
/// Vaulter itself among the creatures tapped to pay for the crew or the saddle, on its controller's turn in a precombat or
/// postcombat main phase, as it is tapped. The Mount or Vehicle is the subject. Another creature crewing it does not trigger it,
/// and neither does crewing in combat (crew has no timing restriction, saddle is sorcery speed).
fn this_saddles_or_crews_in_your_main_phase(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (Event::Saddled { creature, mount: permanent } | Event::Crewed { creature, vehicle: permanent }) = event else { return None };
    let turn = q.state().turn;
    if *creature != me.source || turn.active != me.controller || !turn.step().is_main() {
        return None;
    }
    Fired::about(Target::Object(*permanent))
}

/// "That Mount or Vehicle gains flying until end of turn." (CR 611.2a, 702.9a): does nothing if it has left the battlefield since
/// (CR 400.7).
fn gains_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = item.subject.object().filter(|&o| g.is_on_battlefield(o)) {
        gain_until_end_of_turn(g, item.object, permanent, Keyword::Flying);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::SADDLED.0 | EventMask::CREWED.0), this_saddles_or_crews_in_your_main_phase),
        gains_flying,
        "Whenever this creature saddles a Mount or crews a Vehicle during your main phase, that Mount or Vehicle gains flying until end of turn.",
    )],
    ..oracle::CANYON_VAULTER
};
