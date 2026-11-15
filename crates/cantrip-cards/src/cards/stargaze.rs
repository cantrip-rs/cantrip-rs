use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_take_some_rest_to_graveyard;
use cantrip_oracle as oracle;

/// "Look at twice X cards from the top of your library. Put X cards from among them into your hand and the rest into your graveyard.
/// You lose X life." (CR 107.3a, 701.20e, 400.3): X is the one chosen as it was cast, so twice X cards are looked at, exactly X of
/// them taken (all of a shorter library, CR 609.3) and the rest binned together; then X life is lost whatever the library held.
fn look_at_twice_x_take_x_and_lose_x_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_take_some_rest_to_graveyard(g, item.controller, item.object, 2 * item.x as usize, item.x)?;
    g.lose_life(item.controller, item.x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], look_at_twice_x_take_x_and_lose_x_life),
    ..oracle::STARGAZE
};
