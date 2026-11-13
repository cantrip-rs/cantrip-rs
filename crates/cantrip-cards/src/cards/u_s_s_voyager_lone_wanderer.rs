use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, statics, station};
use cantrip_oracle as oracle;

const EIGHT_PLUS: &str = "8+ | Flying, vigilance (3/3)";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            |g, item| library::reveal_until_two_lands_onto_battlefield_tapped(g, item.controller),
            "When U.S.S. Voyager enters, reveal cards from the top of your library until you reveal two land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
        ),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<8>, station::is_a_creature, EIGHT_PLUS),
        static_ability(Layer::Ability, station::charged::<8>, statics::have_flying_and_vigilance, EIGHT_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<8>, statics::base_pt::<3, 3>, EIGHT_PLUS).continues(),
    ],
    ..oracle::U_S_S_VOYAGER_LONE_WANDERER
};
