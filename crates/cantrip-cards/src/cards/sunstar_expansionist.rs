use cantrip_api::*;
use cantrip_mechanics::tokens::{self, LANDER};
use cantrip_mechanics::triggers::{LANDFALL, THIS_ENTERS};
use cantrip_mechanics::{an_opponent_controls_more_lands, effects};
use cantrip_oracle as oracle;

/// "When this creature enters, if an opponent controls more lands than you, create a Lander token" (CR 603.4).
fn lander(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_lander(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, lander, "When this creature enters, if an opponent controls more lands than you, create a Lander token.")
            .condition(an_opponent_controls_more_lands),
        triggered(
            LANDFALL,
            effects::this_gets::<1, 0>,
            "Landfall \u{2014} Whenever a land you control enters, this creature gets +1/+0 until end of turn.",
        ),
    ],
    ..oracle::SUNSTAR_EXPANSIONIST
};

pub static EXTRA: &[&CardDef] = &[&LANDER];
