use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target monocolored permanent" (CR 115.1): a permanent that is exactly one color. Colorless permanents are not
/// monocolored (CR 105.2a, 105.2c; ruling 2024-04-12), nor are those of two or more colors (CR 105.2b).
fn monocolored_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.chars(id).colors.is_monocolored())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(monocolored_permanent)], effects::exile_first_target),
    ..oracle::VANISHING_VERSE
};
