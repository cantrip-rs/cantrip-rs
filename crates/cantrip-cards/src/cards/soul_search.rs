use cantrip_api::*;
use cantrip_mechanics::{hand::exile_chosen_nonland_from_revealed_hand, targets, tokens};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland card from it. Exile that card. If the card's mana value is 1 or less,
/// create a 1/1 white and black Spirit creature token with flying." The mana value is the exiled card's, read where it now is (an
/// X in its cost is 0 outside the stack, CR 202.3e); a card that did not get exiled gives no token. An opponent who is no longer a
/// legal target as this resolves shows and loses nothing, and then there is no card to ask about.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let exiled = exile_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    if exiled.iter().any(|&card| g.mana_value(card) <= 1) {
        tokens::create(g, item.controller, &tokens::WHITE_BLACK_FLYING_SPIRIT, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], effect), ..oracle::SOUL_SEARCH };

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLACK_FLYING_SPIRIT];
