use cantrip_api::*;
use cantrip_mechanics::chosen_type::enters_choosing_a_creature_type;
use cantrip_oracle as oracle;

/// "Double all damage that sources you control of the chosen type would deal" (CR 614.1, 120.3, 205.3m): damage from any
/// source its controller controls that has the chosen creature type as the damage is dealt - a creature, or a spell or
/// permanent that has the type - combat or not, to anything. Damage from another player's source, or of another type, is
/// not changed, and a source that is not of the type is not (the type is chosen as this enters, CR 614.12a).
fn damage_from_a_source_of_the_chosen_type(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    let Proposal::Damage(d) = p else { return false };
    d.amount > 0
        && q.state().chosen_subtype(q.linked_to(asker)).is_some_and(|t| {
            q.state().obj(d.source).controller == asker.controller && q.chars(d.source).has_subtype(t)
        })
}

fn double(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount *= 2;
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    replacements: &[replacement(ReplacementKind::Damage, damage_from_a_source_of_the_chosen_type, double, "double all damage that sources you control of the chosen type would deal")],
    ..oracle::COLLECTIVE_INFERNO
};
