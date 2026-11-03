use cantrip_api::*;
use cantrip_mechanics::{effects, mobilize, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mobilize::trigger::<1>("Mobilize 1 (Whenever this creature attacks, create a tapped and attacking 1/1 red Warrior creature token. Sacrifice it at the beginning of the next end step.)"),
        ability(cost("{1}{R}").and(SACRIFICE_THIS), effects::damage_first_target_by_creatures_you_control, "{1}{R}, Sacrifice this creature: It deals damage equal to the number of creatures you control to target creature.")
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[mobilize::SACRIFICE],
    ..oracle::STADIUM_HEADLINER
};

pub static EXTRA: &[&CardDef] = &[&mobilize::WARRIOR];
