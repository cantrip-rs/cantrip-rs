use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you cast a spell that targets this creature" (CR 603.2, 601.2c): a question about the spell as it is cast, whose
/// targets are public from the moment it was announced. One trigger for the cast however many times the spell targets this
/// permanent. A copy is not cast (CR 707.10), so it sets nothing off; a spell another player casts does not.
fn you_cast_a_spell_that_targets_this(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if let Event::SpellCast { spell, player, .. } = event
        && *player == me.controller
        && let Some(item) = q.state().stack.iter().find(|i| i.kind == StackKind::Spell && i.object == *spell)
        && q.state().targets(item).contains(&Target::Object(me.source))
    {
        Fired::PLAIN
    } else {
        None
    }
}

/// Trample is a core keyword. "Whenever you cast a spell that targets this creature, draw a card." (CR 603.2, 601.2c, 121.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_that_targets_this),
        |g, item| effects::draw(g, item, 1),
        "Whenever you cast a spell that targets this creature, draw a card.",
    )],
    ..oracle::GNARLBACK_RHINO
};
