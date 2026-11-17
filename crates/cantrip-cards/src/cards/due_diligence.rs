use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{have_vigilance, plus_pt};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control other than enchanted creature" (CR 115.1): a creature on the battlefield under the Aura's
/// controller's control but the one this Aura is attached to.
fn creature_you_control_other_than_enchanted(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(asker.source).attached_to != Some(id))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::targets_get_and_gain(g, item, 2, 2, Keyword::Vigilance),
        "When this Aura enters, target creature you control other than enchanted creature gets +2/+2 and gains vigilance until end of turn.",
    )
    .targets(&[target(creature_you_control_other_than_enchanted)])],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Enchanted creature gets +2/+2 and has vigilance."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Enchanted creature gets +2/+2 and has vigilance."),
    ],
    ..oracle::DUE_DILIGENCE
};
