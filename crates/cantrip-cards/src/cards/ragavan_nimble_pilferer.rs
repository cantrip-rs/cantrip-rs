use cantrip_api::*;
use cantrip_mechanics::dash;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that
/// card (not play: a land exiled this way stays in exile). The permission belongs to Ragavan's controller.
fn treasure_and_steal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::TREASURE, 1)?;
    let Target::Player(whose) = item.subject else { return Ok(()) };
    exile_top_may_play(g, whose, 1, item.controller, PlayGrant { expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, treasure_and_steal, "Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.")],
    ways: &[dash::way(mana("{1}{R}"), "Dash {1}{R}")],
    end_step_riders: dash::RIDERS,
    delayed: &[dash::RETURN],
    statics: &[dash::HASTE],
    ..oracle::RAGAVAN_NIMBLE_PILFERER
};
