use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Each opponent discards a card. You create a 1/1 black and green Pest creature token with 'Whenever this token attacks, you
/// gain 1 life.'" Each opponent chooses their own card, and the Pest is made whether or not anyone had a card to discard.
fn discard_then_pest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_discards_a_card(g, item)?;
    tokens::create_a_pest_that_gains_life_on_attack(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], discard_then_pest), ..oracle::SEND_IN_THE_PEST };

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK];
