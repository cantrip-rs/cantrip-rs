use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature or Dragon spell" (CR 601.2i, 603.2): one cast event triggers once, whether or not the
/// spell is both; Dragon is read through this card's words (CR 612.2).
fn noncreature_or_dragon_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller
                && (!q.chars(*spell).is(CardType::Creature) || q.chars(*spell).has_subtype(lands::word(q, me.source, lands::sub(Subtype::Dragon)))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, noncreature_or_dragon_spell_you_cast),
        effects::damage_each_opponent_n::<1>,
        "Whenever you cast a noncreature or Dragon spell, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::FIRESPITTER_WHELP
};
