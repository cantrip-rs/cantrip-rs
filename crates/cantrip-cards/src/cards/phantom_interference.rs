use cantrip_api::*;
use cantrip_mechanics::{spree, targets, tokens};
use cantrip_oracle as oracle;

fn create_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WHITE_FLYING_SPIRIT_2_2, 1)?;
    Ok(())
}

fn counter_unless_its_controller_pays_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counter_unless_pays(g, item, 0, mana("{2}"))?;
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(create_a_spirit, "+ {3} — Create a 2/2 white Spirit creature token with flying.").cost(mana("{3}")),
    mode(counter_unless_its_controller_pays_two, "+ {1} — Counter target spell unless its controller pays {2}.")
        .targets(&[targets::SPELL])
        .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::PHANTOM_INTERFERENCE };

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT_2_2];
