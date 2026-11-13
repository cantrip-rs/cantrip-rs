use cantrip_api::*;
use cantrip_mechanics::{delayed, manifest_dread, targets, triggers};
use cantrip_oracle as oracle;

/// "Manifest dread" (CR 701.62a), by the spell's controller.
fn manifest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    manifest_dread::manifest_dread(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], delayed::target_creature_gets_plus_power_and_a_delayed_ability_for_when_it_dies::<3>),
    delayed: &[triggered(triggers::THIS_DIES, manifest, "When it dies this turn, manifest dread.")],
    ..oracle::TURN_INSIDE_OUT
};
