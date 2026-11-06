use cantrip_api::*;
use cantrip_mechanics::{behold, tokens};
use cantrip_oracle as oracle;

/// "{2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens." (CR 701.7a).
fn create_three_1_1_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GOBLIN, 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{R}").and(sacrifice(2, behold::goblin)),
            create_three_1_1_red,
            "{2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens.",
        ),
    ],
    ..oracle::GOBLIN_WARRENS
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
