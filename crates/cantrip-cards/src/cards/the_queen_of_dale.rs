use cantrip_api::*;
use cantrip_mechanics::{recruit, tokens};
use cantrip_oracle as oracle;

/// "Whenever an opponent casts their first noncreature spell each turn" History includes casts before the Queen
/// entered and excludes the current cast.
fn opponents_first_noncreature_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    if *player == me.controller || q.chars(*spell).is(CardType::Creature) {
        return None;
    }
    if !cantrip_mechanics::history::first_noncreature_spell_this_turn(q, *spell, *player) {
        None
    } else {
        Fired::PLAIN
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponents_first_noncreature_spell),
        recruit::effect,
        "Whenever an opponent casts their first noncreature spell each turn, you recruit.",
    )],
    ..oracle::THE_QUEEN_OF_DALE
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
