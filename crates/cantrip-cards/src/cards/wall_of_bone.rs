use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{B}: Regenerate this creature." (CR 701.19a).
fn regenerate_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::regenerate::this(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{B}"), regenerate_creature, "{B}: Regenerate this creature."),
    ],
    ..oracle::WALL_OF_BONE
};
