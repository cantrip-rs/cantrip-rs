use cantrip_api::*;
use cantrip_mechanics::{each_discards_cards, effects};
use cantrip_oracle as oracle;

/// "Each player discards two cards." (CR 701.9, 101.4): each player in turn order from the active player chooses two cards from their
/// hand, or all they have, and every card chosen is discarded together.
fn each_player_discards_two_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    each_discards_cards(g, &players, item.object, 2)?;
    Ok(())
}

/// "Each player loses 4 life." (CR 119.3, 101.4): every player in the game loses it together, as one event, replaced in turn order from
/// the active player.
fn each_player_loses_4_life(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let losses: Vec<(PlayerId, i32)> = g.state().apnap().map(|player| (player, 4)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

/// "Choose one or more" (CR 700.2): the chosen modes are done in the order printed, each its own instruction.
static MODAL: ModalDef = modes(&[
    mode(each_player_discards_two_cards, "Each player discards two cards."),
    mode(each_player_loses_4_life, "Each player loses 4 life."),
    mode(effects::each_player_sacrifices_creatures::<2>, "Each player sacrifices two creatures of their choice."),
])
.choose(1, 3);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::RANKLES_PRANK };
