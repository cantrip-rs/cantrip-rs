use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{disguise, tokens};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a)
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, investigate, "Whenever this creature deals combat damage to a player, investigate.")],
    ways: &[disguise::way(Cost::mana(mana("{3}{G/U}{G/U}")), "Disguise {3}{G/U}{G/U}")],
    ..oracle::UNDERCOVER_CROCODELF
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
