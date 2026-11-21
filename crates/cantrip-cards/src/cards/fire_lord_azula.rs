use cantrip_api::*;
use cantrip_mechanics::{copy, firebending};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell while Fire Lord Azula is attacking" (CR 603.2, 508.1k, 601.2i): a spell its controller casts while
/// the source is an attacking creature, about that spell ("that spell"). Azula attacking is read as the spell is cast, as the
/// trigger event happens, not again as the ability resolves: it is no intervening "if" (CR 603.4).
fn you_cast_a_spell_while_this_is_attacking(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.is_attacking(me.source) => Fired::about(Target::Object(*spell)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<2>("Firebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)"),
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_spell_while_this_is_attacking),
            copy::copy_the_cast_spell,
            "Whenever you cast a spell while Fire Lord Azula is attacking, copy that spell. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)",
        ),
    ],
    ..oracle::FIRE_LORD_AZULA
};
