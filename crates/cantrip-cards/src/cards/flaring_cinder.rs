use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When this creature enters and whenever you cast a spell with mana value 4 or greater" (CR 603.2, 603.6a): this permanent
/// entering, or a spell its controller casts whose mana value is 4 or more (an X in its cost counts as the X it was cast with
/// while it is on the stack, CR 202.3e).
fn enters_or_you_cast_a_big_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == me.source && to.is(ZoneKind::Battlefield) => Fired::PLAIN,
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.mana_value(*spell) >= 4 => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::SPELL_CAST.0), enters_or_you_cast_a_big_spell),
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "When this creature enters and whenever you cast a spell with mana value 4 or greater, you may discard a card. If you do, draw a card.",
    )],
    ..oracle::FLARING_CINDER
};
