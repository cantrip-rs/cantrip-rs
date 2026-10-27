use cantrip_api::*;
use cantrip_mechanics::mayhem;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[mayhem::way(mana("{4}{R}"), "Mayhem {4}{R}")],
    ..oracle::ABOMINATION_WORLD_RAVAGER
};
