use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{G}{G}, {T}: Destroy target Aura attached to a land." (CR 115.1).
fn destroy_target_aura_attached_land_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura)) && q.state()
        .obj(id).attached_to.is_some_and(| host | q.state().battlefield().iter().copied()
        .filter(| & id | q.chars(id).is(CardType::Land)).collect:: < Vec < ObjectId > >
        ().contains(& host))))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{G}{G}").and(TAP),
                effects::destroy_first_target,
                "{G}{G}, {T}: Destroy target Aura attached to a land.",
            )
            .targets(&[target(destroy_target_aura_attached_land_target_0)]),
    ],
    ..oracle::SAVAEN_ELVES
};
