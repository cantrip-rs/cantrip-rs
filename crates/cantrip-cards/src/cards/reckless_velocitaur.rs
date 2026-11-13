use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_SADDLES_OR_CREWS_IN_YOUR_MAIN_PHASE;
use cantrip_oracle as oracle;

/// "That Mount or Vehicle gets +2/+0 and gains trample until end of turn" (CR 611.2a): one instruction on the permanent the trigger
/// is about; nothing if it is no longer the permanent it was on the battlefield (CR 400.7).
fn that_gets_plus_two_and_tramples(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = item.subject.object() else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, permanent, 2, 0);
        effects::gain_until_end_of_turn(g, item.object, permanent, Keyword::Trample);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_SADDLES_OR_CREWS_IN_YOUR_MAIN_PHASE,
        that_gets_plus_two_and_tramples,
        "Whenever this creature saddles a Mount or crews a Vehicle during your main phase, that Mount or Vehicle gets +2/+0 and gains trample until end of turn.",
    )],
    ..oracle::RECKLESS_VELOCITAUR
};
