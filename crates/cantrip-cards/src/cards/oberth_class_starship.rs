use cantrip_api::*;
use cantrip_mechanics::{effects, statics, station, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const SIX_PLUS: &str = "6+ | Flying";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::search_for_a_basic_plains_card,
            "When this Spacecraft enters, search your library for a basic Plains card, reveal it, put it into your hand, then shuffle."),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<6>, station::is_a_creature, SIX_PLUS),
        static_ability(Layer::Ability, station::charged::<6>, statics::have_flying, SIX_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<6>, statics::base_pt::<2, 2>, SIX_PLUS).continues(),
    ],
    ..oracle::OBERTH_CLASS_STARSHIP
};
