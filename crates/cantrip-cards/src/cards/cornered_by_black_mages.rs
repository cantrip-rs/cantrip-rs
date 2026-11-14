use cantrip_api::*;
use cantrip_mechanics::{first_target_player_sacrifices_a_creature, targets, tokens};
use cantrip_oracle as oracle;

/// "Target opponent sacrifices a creature of their choice. Create a 0/1 black Wizard creature token ..." (CR 701.21a, 111.1): you get the token whether or not they had one to
/// sacrifice (it is a separate sentence, not conditioned on the sacrifice).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    first_target_player_sacrifices_a_creature(g, item)?;
    tokens::create(g, item.controller, &tokens::WIZARD, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], effect), ..oracle::CORNERED_BY_BLACK_MAGES };

pub static EXTRA: &[&CardDef] = &[&tokens::WIZARD];
