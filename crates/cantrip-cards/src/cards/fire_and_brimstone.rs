use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// A player who attacked this turn (CR 508.1).
fn player_who_attacked(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(p) if q.history().attacked_with(p).next().is_some())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(player_who_attacked)], effects::damage_target_and_you::<4, 4>),
    ..oracle::FIRE_AND_BRIMSTONE
};
