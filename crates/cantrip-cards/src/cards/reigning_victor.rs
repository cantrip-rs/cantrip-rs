use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, mobilize, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mobilize::trigger::<1>("Mobilize 1 (Whenever this creature attacks, create a tapped and attacking 1/1 red Warrior creature token. Sacrifice it at the beginning of the next end step.)"),
        triggered(THIS_ENTERS, effects::target_creature_gets_and_gains_indestructible::<1, 0>, "When this creature enters, target creature gets +1/+0 and gains indestructible until end of turn.").targets(&[targets::CREATURE]),
    ],
    delayed: &[mobilize::SACRIFICE],
    ..oracle::REIGNING_VICTOR
};

pub static EXTRA: &[&CardDef] = &[&mobilize::WARRIOR];
