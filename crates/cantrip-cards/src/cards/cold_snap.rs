use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, triggers::EACH_PLAYERS_UPKEEP};
use cantrip_oracle as oracle;

/// Count that player's snow lands as the ability resolves (CR 608.2h).
fn snow_land_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    let amount = g.state().battlefield().iter().copied().filter(|&id| {
        g.chars(id).supertypes.snow && g.chars(id).is(CardType::Land)
            && g.controller(id) == player
    }).count() as u32;
    g.damage(item.object, Target::Player(player), amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{2}")), "Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        triggered(
            EACH_PLAYERS_UPKEEP,
            snow_land_damage,
            "At the beginning of each player's upkeep, this enchantment deals damage to that player equal to the number of snow lands they control.",
        ),
    ],
    ..oracle::COLD_SNAP
};
