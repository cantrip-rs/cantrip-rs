use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you cast a spell during an opponent's turn" (CR 603.2, 601.2i): a spell cast (not copied, CR 707.10) by this permanent's
/// controller while the turn is another player's.
fn you_cast_a_spell_during_an_opponents_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player == me.controller && q.state().turn.active != me.controller => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_during_an_opponents_turn),
        |g, item| effects::surveil(g, item, 2),
        "Whenever you cast a spell during an opponent's turn, surveil 2.",
    )],
    ..oracle::UNWELCOME_SPRITE
};
