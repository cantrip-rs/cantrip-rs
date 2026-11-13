use cantrip_api::*;
use cantrip_mechanics::{behold, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{R}").and(sacrifice(2, behold::goblin)),
            tokens::create_goblins::<3>,
            "{2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens.",
        ),
    ],
    ..oracle::GOBLIN_WARRENS
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
