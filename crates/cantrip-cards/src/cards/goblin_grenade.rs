use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Goblin may be any permanent with that subtype (CR 118.8, 601.2h).
fn as_additional_cost_cast_spell(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effects::damage_first_target::<5>),
    additional_cost: Some(sacrifice(1, as_additional_cost_cast_spell)),
    ..oracle::GOBLIN_GRENADE
};
