use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Return up to one target creature card from your graveyard to your hand. Create a 1/1 red Mercenary creature token ..." (CR 400.3,
/// 111.1): with no target chosen the token is still made; if the one chosen became illegal the spell does not resolve at all
/// (CR 608.2b), the core's.
fn return_and_create_a_mercenary(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    g.end_sentence();
    tokens::create_a_mercenary(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.optional()], return_and_create_a_mercenary),
    ..oracle::MOURNERS_SURPRISE
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
