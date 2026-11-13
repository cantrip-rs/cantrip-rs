use cantrip_api::*;
use cantrip_mechanics::{artifacts_you_control, targets, tokens, triggers};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// "Target creature gets +X/+0 until end of turn, where X is the number of artifacts you control": X is counted once, as
/// the ability resolves (CR 608.2h), and the target is pumped if it is still legal (CR 608.2b).
fn pump_by_artifacts(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let x = artifacts_you_control(g, item.controller) as i32;
        cantrip_mechanics::pump(g, item.object, creature, x, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_a_thopter, "When Pia enters, create a 1/1 colorless Thopter artifact creature token with flying."),
        ability(
            cost("{5}{R}"),
            pump_by_artifacts,
            "{5}{R}: Target creature gets +X/+0 until end of turn, where X is the number of artifacts you control.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::PIA_DETERMINED_REBUILDER
};

pub static EXTRA: &[&CardDef] = &[&THOPTER];
