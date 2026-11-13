use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The permanent tapped for the cost is the enchanted land (CR 118.1).
fn enchanted_land(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.attached_to(asker.source) == Some(id)
}

/// The ability can be activated only while the enchanted land is untapped (CR 602.5).
fn land_untapped(q: &dyn Query, asker: &Asker) -> bool {
    q.attached_to(asker.source).is_some_and(|id| !q.obj(id).tapped)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, enchanted_land),
            effects::target_creature_gets::<1, 2>,
            "Tap enchanted land: Target blocking creature gets +1/+2 until end of turn. Activate only if enchanted land is untapped.",
        )
        .targets(&[targets::BLOCKING_CREATURE])
        .condition(land_untapped),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::EARTHLORE
};
