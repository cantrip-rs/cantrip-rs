use cantrip_api::*;
use cantrip_mechanics::prepare::target_becomes_prepared as prepare;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature that attacked this turn" (CR 508.1, 115.1): asked of the turn's history, so a creature no longer
/// attacking still counts.
fn attacked_this_turn(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.history().attacked_this_turn(id))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{2}, {T}"),
            prepare,
            "{2}, {T}: Target creature that attacked this turn becomes prepared. Activate only as a sorcery.",
        )
        .targets(&[target(attacked_this_turn)])
        .sorcery_speed(),
        ability(cost("{4}, {T}"), prepare, "{4}, {T}: Target creature becomes prepared.").targets(&[targets::CREATURE]),
    ],
    ..oracle::HEXHAVEN_DUELING_ARENA
};
