use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}: Destroy target Aura attached to a creature you control." (CR 115.1).
fn destroy_target_aura_attached_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(t, Target::Object(aura) if targets::permanent(q, aura)
        && q.chars(aura).has_subtype(Subtype::Aura)
        && q.obj(aura).attached_to.is_some_and(|host| targets::permanent(q, host)
            && q.chars(host).is(CardType::Creature) && q.controller(host) == asker.controller))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                effects::destroy_first_target,
                "{T}: Destroy target Aura attached to a creature you control.",
            )
            .targets(&[target(destroy_target_aura_attached_creature_target_0)]),
    ],
    ..oracle::MIRACLE_WORKER
};
