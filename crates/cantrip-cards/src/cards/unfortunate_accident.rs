use cantrip_api::*;
use cantrip_mechanics::{effects, spree, targets, tokens};
use cantrip_oracle as oracle;

const MODES: &[ModeDef] = &[
    mode(effects::destroy_first_target, "+ {2}{B} — Destroy target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{2}{B}")),
    mode(
        tokens::create_a_mercenary,
        "+ {1} — Create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
    )
    .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::UNFORTUNATE_ACCIDENT };
