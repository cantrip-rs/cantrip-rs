use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_cards_and_create_a_treasure::<2>),
    // As an additional cost to cast this spell, sacrifice an artifact or creature.
    additional_cost: Some(Cost::sacrifice(1, cantrip_mechanics::targets::filters::artifact_or_creature)),
    ..oracle::DEADLY_DISPUTE
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
