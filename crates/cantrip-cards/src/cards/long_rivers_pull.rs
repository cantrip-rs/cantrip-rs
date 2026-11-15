use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets};
use cantrip_oracle as oracle;

/// "Counter target creature spell. If the gift was promised, instead counter target spell." The promise is announced before the
/// target is chosen (CR 601.2b-c, 702.174m), so the target is judged by it, and again as the spell resolves (CR 608.2b).
fn creature_spell_or_any_spell(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    gift::target_by_promise(q, asker, t, &targets::SPELL, &targets::CREATURE_SPELL)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_spell_or_any_spell)], effects::counter_first_target),
    gift: Some(gift::CARD),
    ..oracle::LONG_RIVERS_PULL
};
