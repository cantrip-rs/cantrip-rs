use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::{blight, effects, targets};
use cantrip_oracle as oracle;

/// "{1}{W}, Blight 2: Return this card from your graveyard to your hand. Activate only as a sorcery." Blight 2 (CR 701.68a) puts
/// two -1/-1 counters on one creature its controller controls; it functions only in the graveyard (CR 113.6m), which `from` says.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{1}{W}").and(blight::cost(2)),
        effects::return_this_to_hand,
        "{1}{W}, Blight 2: Return this card from your graveyard to your hand. Activate only as a sorcery.",
    )
    .from(ZoneKind::Graveyard)
    .sorcery_speed()],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_flying, "Enchanted creature has flying."),
    ],
    ..oracle::EVERSHRIKES_GIFT
};
