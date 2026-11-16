use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, crew::crew, lands, statics::{artifact_of_subtype_alone, grant_delayed_ability}, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    statics: &[
        static_ability(Layer::Type, attached_object, artifact_of_subtype_alone::<{ lands::sub(Subtype::Vehicle) }>, "Enchanted permanent is a Vehicle artifact and loses all other card types."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Enchanted permanent has crew 5."),
    ],
    delayed: &[crew(5, "Crew 5 (Tap any number of creatures you control with total power 5 or more: This Vehicle becomes an artifact creature until end of turn.)")],
    ..oracle::SWIFT_RECONFIGURATION
};
