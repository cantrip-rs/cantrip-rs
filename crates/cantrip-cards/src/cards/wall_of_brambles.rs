use cantrip_api::*;
use cantrip_mechanics::regenerate;
use cantrip_oracle as oracle;

/// "{G}: Regenerate this creature." (CR 701.19a).
fn regenerate_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::this(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{G}"), regenerate_creature, "{G}: Regenerate this creature."),
    ],
    ..oracle::WALL_OF_BRAMBLES
};
