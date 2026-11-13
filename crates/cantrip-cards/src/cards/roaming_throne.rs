use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::chosen_type::{add_chosen_type, enters_choosing_a_creature_type};
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

/// "If a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time."
/// (CR 603.2d): the source judged as it is when the ability triggers (as it last existed for one that looks back).
fn another_creature_of_chosen_type(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    let Some(chosen) = q.state().chosen_subtype(q.linked_to(asker)) else { return false };
    triggering.of_a_permanent_controlled_by(asker.controller) && triggering.source != asker.source && {
        let c = q.chars(triggering.source);
        c.is(CardType::Creature) && c.has_subtype(chosen)
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    abilities: &[ward::<2>("Ward {2}")],
    statics: &[static_ability(Layer::Type, itself, add_chosen_type, "This creature is the chosen type in addition to its other types.")],
    additional_triggers: &[another_creature_of_chosen_type],
    ..oracle::ROAMING_THRONE
};
