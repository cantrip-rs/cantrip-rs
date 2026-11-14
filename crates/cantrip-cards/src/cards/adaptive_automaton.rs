use cantrip_api::*;
use cantrip_mechanics::chosen_type::{add_chosen_type, enters_choosing_a_creature_type, other_creatures_you_control_of_chosen_type};
use cantrip_mechanics::statics::{itself, plus_pt};
use cantrip_oracle as oracle;

/// "As this creature enters, choose a creature type." (CR 614.12a, 205.3m), kept on the permanent (CR 607.2d).
/// "This creature is the chosen type in addition to its other types" (CR 205.1b, 613.1d) and "Other creatures you control of the
/// chosen type get +1/+1" (CR 613.4c): the Automaton is left out of the second, though it is the chosen type itself.
pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    statics: &[
        static_ability(Layer::Type, itself, add_chosen_type, "This creature is the chosen type in addition to its other types."),
        static_ability(Layer::PtModify, other_creatures_you_control_of_chosen_type, plus_pt::<1, 1>, "Other creatures you control of the chosen type get +1/+1."),
    ],
    ..oracle::ADAPTIVE_AUTOMATON
};
