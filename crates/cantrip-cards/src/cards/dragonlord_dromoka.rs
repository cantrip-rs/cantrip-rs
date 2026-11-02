use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Your opponents can't cast spells during your turn." (CR 604.1).
fn opponents_cant_cast_spells_during_rule(
    q: &dyn Query,
    asker: &Asker,
    player: PlayerId,
    _: ObjectId,
) -> bool {
    !((asker.controller == q.state().turn.active) && player != asker.controller)
}

pub static CARD: CardDef = CardDef {
    spell_rules: &[opponents_cant_cast_spells_during_rule],
    ..oracle::DRAGONLORD_DROMOKA
};
