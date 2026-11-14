use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("PutPlusOneCounter", "put a +1/+1 counter on this creature?", "", "Put a counter", "Don't");

/// "Whenever an opponent casts a spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10), by a player other than the
/// ability's controller.
fn opponent_casts_a_spell(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player != me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "You may put a +1/+1 counter on this creature" (CR 122.1): asked as the ability resolves, so it resolves whether or not the
/// spell that caused it still exists (the ruling), and nothing is asked once this creature has left the battlefield (CR 400.7).
fn may_put_a_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may_this_card(g, item, Purpose::Ask(&ASK))? {
        effects::plus_one_counter_on_this(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_spell),
        may_put_a_counter_on_this,
        "Whenever an opponent casts a spell, you may put a +1/+1 counter on this creature.",
    )],
    ..oracle::TAUREAN_MAULER
};
