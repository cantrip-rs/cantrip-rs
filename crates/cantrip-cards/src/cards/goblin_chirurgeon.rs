use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

/// Regenerate target creature (CR 701.19a).
fn regenerate_target_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

/// A Goblin sacrificed as the activation cost (CR 701.21a).
fn goblin(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(sacrifice(1, goblin), regenerate_target_creature, "Sacrifice a Goblin: Regenerate target creature.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::GOBLIN_CHIRURGEON
};
