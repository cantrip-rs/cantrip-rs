use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a white or blue instant or sorcery spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10),
/// by a player other than this ability's controller, that is an instant or a sorcery and is white, blue or both as it stands on the
/// stack (CR 105.2), the colors read through the words of this card's text (CR 612.2). A spell of both colors triggers it once.
/// The trigger is about the player who cast it ("that player").
fn opponent_casts_a_white_or_blue_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let words = q.words(me.source);
    let colors = q.chars(*spell).colors;
    let instant_or_sorcery = q.chars(*spell).is(CardType::Instant) || q.chars(*spell).is(CardType::Sorcery);
    if *player != me.controller && instant_or_sorcery && (colors.has(words.color(Color::White)) || colors.has(words.color(Color::Blue))) {
        return Fired::about(Target::Player(*player));
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_white_or_blue_instant_or_sorcery),
        |g, item| effects::damage_that_player(g, item, 2),
        "Whenever an opponent casts a white or blue instant or sorcery spell, this creature deals 2 damage to that player.",
    )],
    ..oracle::MINDSPARKER
};
