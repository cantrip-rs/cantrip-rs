use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Target non-Mount creature" (CR 115.1, 205.3m): a creature on the battlefield, any controller's, that does not have the
/// creature type Mount as the layers made it (read through the words of this card's text, CR 612.2). Checked again as the
/// ability resolves (CR 608.2b).
fn non_mount_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id)
        && q.chars(id).is(CardType::Creature)
        && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Mount)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}, {T}"), effects::tap_targets, "{2}, {T}: Tap target non-Mount creature.").targets(&[target(non_mount_creature)])],
    ..oracle::STERLING_KEYKEEPER
};
