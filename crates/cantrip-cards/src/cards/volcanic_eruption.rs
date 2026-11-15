use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy X target Mountains. Volcanic Eruption deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way." (CR 115.1).
fn destroy_x_target_mountains_volcanic_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Mountain)))
    )
}

/// "Destroy X target Mountains. Volcanic Eruption deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way." (CR 608.2, 120.3, 701.8a).
/// A Mountain that regenerated, was indestructible or went elsewhere is not put into a graveyard.
fn destroy_x_target_mountains_volcanic(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = cantrip_mechanics::targets::legal_objects(g, item);
    let destroyed = g.destroy_all(&objects_0);
    g.end_sentence();
    let amount = cantrip_mechanics::effects::put_into_a_graveyard(g, &destroyed).len();
    cantrip_mechanics::effects::damage_each_creature_and_player_n(g, item, amount as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(destroy_x_target_mountains_volcanic_target_0).counted()],
        destroy_x_target_mountains_volcanic,
    ),
    ..oracle::VOLCANIC_ERUPTION
};
