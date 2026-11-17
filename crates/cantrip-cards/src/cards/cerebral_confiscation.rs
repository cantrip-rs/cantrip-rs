use cantrip_api::*;
use cantrip_mechanics::hand::discard_chosen_nonland_from_revealed_hand;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card." (CR 701.9a, 608.2b):
/// nothing if the opponent is no longer a legal target as the spell resolves.
fn target_opponent_discards_a_chosen_nonland_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::target_player_discards::<2>, "Target opponent discards two cards.").targets(&[targets::OPPONENT]),
    mode(target_opponent_discards_a_chosen_nonland_card, "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card.")
        .targets(&[targets::OPPONENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CEREBRAL_CONFISCATION };
