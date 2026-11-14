use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Up to one target Equipment attached to that creature" (CR 115.1, 301.5, 601.2c): an Equipment, whoever controls it, attached
/// to the creature the first slot chose. Judged again as the spell resolves (CR 608.2b), so one that has been unattached or
/// has left the battlefield is not exiled.
fn equipment_attached_to_that_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!((t, q.chosen_targets().first()), (Target::Object(id), Some(&Target::Object(creature)))
        if targets::permanent(q, id) && q.chars(id).has_subtype(Subtype::Equipment) && q.attached_to(id) == Some(creature))
}

/// "Fiery Annihilation deals 5 damage to target creature. Exile up to one target Equipment attached to that creature. If that
/// creature would die this turn, exile it instead." (CR 608.2c, 614.1a): in that order. The replacement is the spell's own
/// effect on that creature (CR 611.2c), made once the damage is dealt: the creature has not died yet (state-based actions wait,
/// CR 704.3). The Equipment is exiled only while the creature is still a legal target (ruling 2024-11-08): "that creature" is no longer a
/// creature the spell can act on otherwise, so an illegal creature leaves the Equipment alone.
fn fiery_annihilation(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creature = targets::object(g, item, 0);
    if let Some(creature) = creature {
        g.damage(item.object, Target::Object(creature), 5);
    }
    g.end_sentence();
    if creature.is_some()
        && let Some(equipment) = targets::object(g, item, 1)
    {
        g.exile_all(&[equipment]);
    }
    g.end_sentence();
    if let Some(creature) = creature {
        exile_if_it_would_die(g, item, 0, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE, up_to_one(equipment_attached_to_that_creature)], fiery_annihilation),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::FIERY_ANNIHILATION
};
