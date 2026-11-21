use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target planeswalker" (CR 115.1): a planeswalker on the battlefield.
fn planeswalker(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.chars(id).is(CardType::Planeswalker))
}

/// "As an additional cost to cast this spell, collect evidence X, where X is the total mana value of the permanents this
/// spell targets" (CR 701.59a, 601.2f): worked out once the targets are announced, each permanent counted once however many
/// of the spell's slots chose it (an artifact creature targeted as the artifact and as the creature), at its mana value now.
fn collect_evidence_of_the_targets(q: &dyn Query, _: &Asker, chosen: &[Target]) -> Cost {
    let mut permanents: Vec<ObjectId> = chosen.iter().filter_map(|t| t.object()).collect();
    permanents.sort_unstable();
    permanents.dedup();
    let total: u32 = permanents.iter().map(|&id| q.mana_value(id)).sum();
    Cost { collect_evidence: total, ..Cost::NONE }
}

/// "Destroy up to one target artifact, up to one target creature, up to one target enchantment, and up to one target
/// planeswalker": every target still legal, together (CR 608.2b, 101.4).
fn destroy_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.destroy_all(&targets::legal_objects(g, item));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::ARTIFACT.between(0, 1),
            targets::CREATURE.between(0, 1),
            targets::ENCHANTMENT.between(0, 1),
            target(planeswalker).between(0, 1),
        ],
        destroy_each,
    ),
    cost_of_targets: Some(collect_evidence_of_the_targets),
    ..oracle::URGENT_NECROPSY
};
