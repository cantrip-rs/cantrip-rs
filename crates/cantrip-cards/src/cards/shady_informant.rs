use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies, it deals 2 damage to any target" (CR 700.4, 120.3): the creature is the source, off its last-known
/// information. Face down it is a nameless 2/2 with no abilities (CR 708.2), so it deals nothing when it dies; it deals damage as a
/// face-up card in the graveyard even if turned up (a face-down permanent that leaves is revealed, CR 708.9).
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{2}{B/R}{B/R}")), "Disguise {2}{B/R}{B/R}")],
    abilities: &[triggered(
        triggers::THIS_DIES,
        effects::damage_first_target::<2>,
        "When this creature dies, it deals 2 damage to any target.",
    )
    .targets(&[targets::ANY])],
    ..oracle::SHADY_INFORMANT
};
