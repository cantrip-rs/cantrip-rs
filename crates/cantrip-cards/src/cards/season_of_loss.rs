use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, effects, history};
use cantrip_oracle as oracle;

/// {P}{P} - "Draw a card for each creature that died under your control this turn." (CR 700.4): counted as this mode resolves, so
/// the creatures an earlier {P} sacrificed (modes run in printed order, CR 700.2d) are among them; tokens too.
fn draw_for_each_of_your_creatures_that_died(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let died = history::creatures_died_this_turn(g, Some(item.controller)) as u32;
    effects::draw(g, item, died)
}

/// {P}{P}{P} - "Each opponent loses X life, where X is the number of creature cards in your graveyard." (CR 119.3, 608.2h): X
/// counted as this mode resolves.
fn each_opponent_loses_creature_cards_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creature_cards_in_graveyard(g, item.controller) as i32;
    g.each_opponent_loses_life(item.controller, x);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(effects::each_player_sacrifices_creatures::<1>, "{P} \u{2014} Each player sacrifices a creature of their choice.").pawprints(1),
    mode(draw_for_each_of_your_creatures_that_died, "{P}{P} \u{2014} Draw a card for each creature that died under your control this turn.")
        .pawprints(2),
    mode(
        each_opponent_loses_creature_cards_in_your_graveyard,
        "{P}{P}{P} \u{2014} Each opponent loses X life, where X is the number of creature cards in your graveyard.",
    )
    .pawprints(3),
])
.pawprints(5);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::SEASON_OF_LOSS };
