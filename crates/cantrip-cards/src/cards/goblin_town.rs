use cantrip_api::*;
use cantrip_mechanics::{lands, mana, targets};
use cantrip_oracle as oracle;

/// "Target Goblin or Orc you control" (CR 115.1, 109.5): a permanent on the battlefield under this ability's controller's control
/// with one of those creature types, the words read through this land's own (CR 612).
fn goblin_or_orc_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    let words = q.words(asker.source);
    targets::permanent(q, id)
        && q.controller(id) == asker.controller
        && [Subtype::Goblin, Subtype::Orc].into_iter().any(|s| q.chars(id).has_subtype(words.subtype(s)))
}

/// "This land enters tapped." (CR 614.1c), "{T}: Add {B} or {R}." and "{2}{B}{R}, {T}, Sacrifice this land: Put two +1/+1 counters
/// on target Goblin or Orc you control. Activate only as a sorcery." (CR 602.5d, 122.1), the last as the Hobbit cycle prints it
/// (`lands::sacrifice_for_two_counters`).
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Black, Color::Red]), "{T}: Add {B} or {R}."),
        lands::sacrifice_for_two_counters(
            "{2}{B}{R}, {T}",
            &[target(goblin_or_orc_you_control)],
            "{2}{B}{R}, {T}, Sacrifice this land: Put two +1/+1 counters on target Goblin or Orc you control. Activate only as a sorcery.",
        ),
    ],
    ..oracle::GOBLIN_TOWN
};
