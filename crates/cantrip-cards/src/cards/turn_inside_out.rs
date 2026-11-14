use cantrip_api::*;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], delayed::target_creature_gets_plus_power_and_a_delayed_ability_for_when_it_dies::<3>),
    delayed: &[triggered(triggers::THIS_DIES, controller_manifests_dread, "When it dies this turn, manifest dread.")],
    ..oracle::TURN_INSIDE_OUT
};
