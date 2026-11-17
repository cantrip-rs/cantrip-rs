use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Up to one other target creature token" (CR 115.1, 115.3): a creature token that is not the first target. The same creature
/// can't be both ("other"); the first target is already chosen when this slot is.
fn other_creature_token(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_token_filter(q, asker, t) && !q.chosen_targets().contains(&t)
}

/// "Trick Shot deals 6 damage to target creature and 2 damage to up to one other target creature token" (CR 120.3, 608.2b): the
/// damage is dealt together; a target that has become illegal is left out and the other is still dealt its damage.
fn damage_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u32)> = [(0, 6), (1, 2)]
        .into_iter()
        .filter_map(|(slot, amount)| targets::object(g, item, slot).map(|id| (item.object, Target::Object(id), amount)))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE, up_to_one(other_creature_token)], damage_both),
    ..oracle::TRICK_SHOT
};
