use cantrip_api::*;
use cantrip_oracle as oracle;

/// "When this enchantment enters and whenever you cast a spell with mana value 5 or greater" (CR 603.2).
fn when_enchantment_enters_whenever_cast_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    if let Event::ZoneChange { new, to, .. } = event {
        let id = *new;
        let subject = Target::Object(id);
        if to.is(ZoneKind::Battlefield) && asker.source == id {
            return Fired::about(subject);
        }
    }
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player && (i64::from(q.mana_value(id))) >= 5 {
        return Fired::about(subject);
    }
    None
}

/// "When this enchantment enters and whenever you cast a spell with mana value 5 or greater, draw a card." (CR 608.2, 121.1).
fn draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask(EventMask::ZONE_CHANGE.0 | EventMask::SPELL_CAST.0),
                when_enchantment_enters_whenever_cast_spell,
            ),
            draw_card,
            "When this enchantment enters and whenever you cast a spell with mana value 5 or greater, draw a card.",
        ),
    ],
    ..oracle::UP_THE_BEANSTALK
};
