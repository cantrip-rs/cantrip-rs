use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, return_this_to_the_battlefield_attached_to_first_target};
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature you control with power 1 or less" (CR 115.1, 608.2b): a creature on the battlefield under the activating
/// player's control whose power, as the layers make it, is at most 1 (a negative power too), judged again as the ability resolves.
fn creature_you_control_with_power_one_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).power.is_some_and(|p| p <= 1))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{2}{W/U}{W/U}"),
        return_this_to_the_battlefield_attached_to_first_target,
        "{2}{W/U}{W/U}: Return this card from your graveyard to the battlefield attached to target creature you control with power 1 or less. Activate only as a sorcery.",
    )
    .targets(&[target(creature_you_control_with_power_one_or_less)])
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .from(ZoneKind::Graveyard)
    .sorcery_speed()],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Enchanted creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_flying, "Enchanted creature has flying."),
    ],
    ..oracle::EAGLES_RESCUE
};
