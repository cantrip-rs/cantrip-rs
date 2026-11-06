use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Cast this spell only during an opponent's turn after their upkeep step. Untap all lands you control." (CR 601.3).
fn cast_spell_only_during_opponents_if(q: &dyn Query, asker: &Asker) -> bool {
    q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .into_iter()
        .any(|bound_0: PlayerId| bound_0 == q.state().turn.active)
        && (q.state().turn.step() as u8) > (Step::Upkeep as u8)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], cantrip_mechanics::effects::untap_all_lands_you_control),
    cast_only_if: Some(cast_spell_only_during_opponents_if),
    ..oracle::RESET
};
