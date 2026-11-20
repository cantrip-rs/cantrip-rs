use cantrip_api::*;
use cantrip_mechanics::{effects, statics, station, triggers};
use cantrip_oracle as oracle;

/// The 8+ striation: a 4/4 artifact creature with flying (CR 721.2b).
const EIGHT_PLUS: &str = "8+ | Flying (4/4)";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::draw_then_discard::<2, 1>,
            "When this Spacecraft enters, draw two cards, then discard a card.",
        ),
        station::ability(
            "Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)",
        ),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<8>, station::is_a_creature, EIGHT_PLUS),
        static_ability(Layer::Ability, station::charged::<8>, statics::have_flying, EIGHT_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<8>, statics::base_pt::<4, 4>, EIGHT_PLUS).continues(),
    ],
    ..oracle::UTHROS_SCANSHIP
};
