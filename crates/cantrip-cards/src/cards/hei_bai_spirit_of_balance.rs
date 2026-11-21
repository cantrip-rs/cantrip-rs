use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS_OR_ATTACKS, THIS_LEAVES};
use cantrip_mechanics::{effects, select, targets};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature or artifact. If you do, put two +1/+1 counters on Hei Bai" (CR 608.2c-d): the sacrifice
/// is chosen as the ability resolves and may be declined; the counters come only if a permanent was actually sacrificed, and go
/// on Hei Bai if it is still on the battlefield.
fn sacrifice_to_put_two_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::another_creature_or_artifact_permanents(select::up_to(1)))?.is_empty() {
        effects::two_plus_one_counters_on_this(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            sacrifice_to_put_two_counters_on_this,
            "Whenever Hei Bai enters or attacks, you may sacrifice another creature or artifact. If you do, put two +1/+1 counters on Hei Bai.",
        ),
        // The counters it had as it last was on the battlefield (CR 603.10a), of every kind (CR 122.6).
        triggered(
            THIS_LEAVES,
            effects::put_its_counters_on_target,
            "When Hei Bai leaves the battlefield, put its counters on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::HEI_BAI_SPIRIT_OF_BALANCE
};
