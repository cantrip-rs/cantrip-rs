use cantrip_api::*;
use cantrip_mechanics::{effects, station, statics, targets, triggers};
use cantrip_oracle as oracle;

/// The 10+ striation, "Flying" with the 5/6 power/toughness box (CR 721.2b): while it has ten or more charge counters it
/// has flying and is a creature with base power and toughness 5/6 in addition to its other types - an artifact creature,
/// as its reminder text says. One static ability, applied in each layer it touches (CR 613.6).
const TEN_PLUS: &str = "10+ | Flying (5/6)";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::return_first_target_to_battlefield,
            "When this Spacecraft enters, return target creature or enchantment card from your graveyard to the battlefield.",
        )
        .targets(&[targets::CREATURE_OR_ENCHANTMENT_CARD_IN_YOUR_GRAVEYARD]),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 10+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<10>, station::is_a_creature, TEN_PLUS),
        static_ability(Layer::Ability, station::charged::<10>, statics::have_flying, TEN_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<10>, statics::base_pt::<5, 6>, TEN_PLUS).continues(),
    ],
    ..oracle::RESCUE_SKIFF
};
