use cantrip_api::*;
use cantrip_mechanics::chosen_type::{add_chosen_type, creatures_you_control_of_chosen_type, enters_choosing_a_creature_type};
use cantrip_mechanics::crew;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "Vehicle creatures you control" (CR 301.7, 205.3): a Vehicle that is a creature right now (crewed, say), under the Engine's
/// controller. An uncrewed Vehicle is not one; a creature that is not a Vehicle is not one.
fn your_vehicle_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature) && so_far.has_subtype(Subtype::Vehicle) && q.state().obj(affected).controller == q.state().obj(source).controller
}

/// "Each creature you control of the chosen type other than this Vehicle" (CR 205.3m): the Engine itself is left out, though it
/// is the chosen type whenever it is a creature.
fn other_creatures_you_control_of_chosen_type(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && creatures_you_control_of_chosen_type(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    abilities: &[crew::crew(3, "Crew 3")],
    statics: &[
        static_ability(
            Layer::Type,
            your_vehicle_creatures,
            add_chosen_type,
            "Vehicle creatures you control are the chosen creature type in addition to their other types.",
        ),
        static_ability(
            Layer::PtModify,
            other_creatures_you_control_of_chosen_type,
            plus_pt::<1, 1>,
            "Each creature you control of the chosen type other than this Vehicle gets +1/+1.",
        ),
    ],
    ..oracle::LIFECRAFT_ENGINE
};
