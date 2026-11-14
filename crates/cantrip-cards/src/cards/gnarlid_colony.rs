use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "If this creature was kicked, it enters with two +1/+1 counters on it" (CR 702.33d, 614.1c): it was kicked at most once
/// (a single kicker), and an object put onto the battlefield without being cast was never kicked and enters bare.
fn enters_with_two_counters_if_kicked(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.obj(entering.object).kicked > 0 {
        entering.counters[CounterKind::PlusOne as usize] += 2;
    }
    Ok(())
}

/// "Each creature you control with a +1/+1 counter on it has trample" (CR 613.1f, 702.19), itself included once it has one.
pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{2}{G}")),
    enters: Some(enters_with_two_counters_if_kicked),
    statics: &[static_ability(
        Layer::Ability,
        statics::your_creatures_with_plus_one_counters,
        statics::have_trample,
        "Each creature you control with a +1/+1 counter on it has trample.",
    )],
    ..oracle::GNARLID_COLONY
};
