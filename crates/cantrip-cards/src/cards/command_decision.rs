use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Choose one mode as the spell is cast (CR 700.2a); only the destroy mode has a target (CR 700.2c).
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature with power 4 or greater.")
        .targets(&[targets::CREATURE_WITH_POWER_4_OR_GREATER]),
    mode(|g, item| effects::draw(g, item, 1), "Draw a card."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::COMMAND_DECISION };
