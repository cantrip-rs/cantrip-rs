use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

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
        triggers::THIS_SADDLES_OR_CREWS_IN_YOUR_MAIN_PHASE,
        gains_flying,
        "Whenever this creature saddles a Mount or crews a Vehicle during your main phase, that Mount or Vehicle gains flying until end of turn.",
    )],
    ..oracle::CANYON_VAULTER
};
