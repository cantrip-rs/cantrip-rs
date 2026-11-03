use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::EACH_PLAYERS_UPKEEP};
use cantrip_oracle as oracle;

/// Count that player's Swamps as the ability resolves (CR 608.2h).
fn karma_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = g.state().battlefield().iter().copied().filter(|&id| {
        g.chars(id).has_subtype(g.words(item.object).subtype(Subtype::Swamp))
            && item.subject.player() == Some(g.controller(id))
    }).count() as u16;
    effects::damage_that_player(g, item, amount)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EACH_PLAYERS_UPKEEP,
        karma_damage,
        "At the beginning of each player's upkeep, this enchantment deals damage to that player equal to the number of Swamps they control.",
    )],
    ..oracle::KARMA
};
