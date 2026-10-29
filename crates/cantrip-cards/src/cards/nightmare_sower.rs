use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell during an opponent's turn" (CR 601.2i, 603.2): a spell its controller cast while another player
/// was the active player.
fn you_cast_a_spell_during_an_opponents_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.state().turn.active == me.controller { None } else { triggers::you_cast_a_spell(q, me, event) }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_during_an_opponents_turn),
        effects::minus_one_counters_on_target::<1>,
        "Whenever you cast a spell during an opponent's turn, put a -1/-1 counter on up to one target creature.",
    )
    .targets(&[targets::CREATURE.optional()])],
    ..oracle::NIGHTMARE_SOWER
};
