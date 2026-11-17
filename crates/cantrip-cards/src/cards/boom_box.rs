use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Destroy up to one target artifact, up to one target creature, and up to one target land" (CR 701.8, 608.2b): three separate
/// targets, each optional, destroyed together as one action. Each is judged as the ability resolves, so one that is gone or no
/// longer legal does not stop the others; one object chosen for two of the slots (an artifact creature) is destroyed once.
fn destroy_the_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut doomed: Vec<ObjectId> = Vec::new();
    for spec in 0..3 {
        let chosen = g.state().spec_targets(item, spec).filter_map(|slot| targets::object(g, item, slot)).collect::<Vec<_>>();
        for permanent in chosen {
            if !doomed.contains(&permanent) {
                doomed.push(permanent);
            }
        }
    }
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{6}").and(TAP).and(SACRIFICE_THIS),
        destroy_the_targets,
        "{6}, {T}, Sacrifice this artifact: Destroy up to one target artifact, up to one target creature, and up to one target land.",
    )
    .targets(&[targets::ARTIFACT.optional(), targets::CREATURE.optional(), targets::LAND.optional()])],
    ..oracle::BOOM_BOX
};
