use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// That creature's last controller sacrifices a land (CR 608.2h, 701.21a).
fn sacrifice_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    if !g.state().in_game(player) {
        return Ok(());
    }
    cantrip_mechanics::sacrifice_chosen(g, player, item.object, 1, |c| c.is(CardType::Land))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            |g, item| effects::sacrifice_this_unless_pay(g, item, cost("{2}")),
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {2}.",
        ),
        triggered(
            triggers::CREATURE_DIES,
            sacrifice_land,
            "Whenever a creature dies, that creature's controller sacrifices a land of their choice.",
        ),
    ],
    ..oracle::EARTHLINK
};
