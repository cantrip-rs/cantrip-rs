use cantrip_api::*;
use cantrip_mechanics::spree;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const MODES: &[ModeDef] = &[
    mode(effects::counter_first_target, "+ {1}{U} — Counter target spell.")
        .targets(&[targets::SPELL])
        .cost(mana("{1}{U}")),
    mode(cantrip_mechanics::copy::create_token_copy_of_target, "+ {3} — Create a token that's a copy of target artifact or creature you control.")
        .targets(&[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL])
        .cost(mana("{3}")),
    mode(effects::draw_then_discard::<2, 1>, "+ {2} — Draw two cards, then discard a card.")
        .cost(mana("{2}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::THREE_STEPS_AHEAD };
