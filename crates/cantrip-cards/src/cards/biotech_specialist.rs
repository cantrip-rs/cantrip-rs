use cantrip_api::*;
use cantrip_mechanics::tokens::{self, LANDER};
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, create a Lander token."
fn lander(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_lander(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, lander, "When this creature enters, create a Lander token."),
        triggered(
            triggers::YOU_SACRIFICE_AN_ARTIFACT,
            effects::damage_first_target::<2>,
            "Whenever you sacrifice an artifact, this creature deals 2 damage to target opponent.",
        )
        .on_sacrifice()
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::BIOTECH_SPECIALIST
};

pub static EXTRA: &[&CardDef] = &[&LANDER];
