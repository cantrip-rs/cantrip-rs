use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::noncreature_or_subtype_spell_you_cast::<{ lands::sub(Subtype::Dragon) }>),
        effects::damage_each_opponent_n::<1>,
        "Whenever you cast a noncreature or Dragon spell, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::FIRESPITTER_WHELP
};
