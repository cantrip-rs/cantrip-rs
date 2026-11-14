use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a blue or black spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10), by a player
/// other than this permanent's controller, judged on the spell as it stands on the stack once cast, a spell of either color (or
/// both) counting. The colors are words of this card's text (CR 612.2).
fn opponent_casts_a_blue_or_black_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player != me.controller && {
                let colors = q.chars(*spell).colors;
                let words = q.words(me.source);
                colors.has(words.color(Color::Blue)) || colors.has(words.color(Color::Black))
            } =>
        {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// "You may put a +1/+1 counter on this creature" (CR 122.1, 608.2d): asked as the ability resolves, and only while the Adder is
/// still the permanent that triggered; once it has left the battlefield it is a new object (CR 400.7), so nothing is asked.
fn may_put_a_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) && effects::ask_may(g, item.controller, item.object, Purpose::PutCounters(CounterKind::PlusOne, 1))? {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_blue_or_black_spell),
        may_put_a_counter_on_this,
        "Whenever an opponent casts a blue or black spell, you may put a +1/+1 counter on this creature.",
    )],
    ..oracle::MOLD_ADDER
};
