use cantrip_api::*;
use cantrip_oracle as oracle;

/// "When this enchantment enters" (CR 603.2).
fn when_enchantment_enters(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When this enchantment enters, draw three cards." (CR 608.2, 121.1).
fn draw_three_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    Ok(())
}

/// "Whenever you cast a spell" (CR 603.2).
fn when_cast_spell(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player {
        return Fired::about(subject);
    }
    None
}

/// "Whenever you cast a spell, you lose 1 life." (CR 608.2, 119.3).
fn lose_1_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 1);
    Ok(())
}

/// "When this enchantment becomes the target of a spell or ability" (CR 603.2).
fn when_enchantment_becomes_target_spell_ability(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::BecameTarget { target: Target::Object(target), .. } = event else { return None };
    let id = *target;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When this enchantment becomes the target of a spell or ability, sacrifice it." (CR 608.2, 701.21a).
fn sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    let players_2 = Some(item.controller).into_iter().collect::<Vec<PlayerId>>();
    let mut picks_2: Vec<(PlayerId, ObjectId)> = Vec::new();
    for &object in &objects_2 {
        let controller = g.controller(object);
        if players_2.contains(&controller) && g.can_sacrifice(controller, object) {
            picks_2.push((controller, object));
        }
    }
    g.sacrifice_each(&picks_2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_enchantment_enters),
            draw_three_cards,
            "When this enchantment enters, draw three cards.",
        ),
        triggered(
            when_asked(EventMask::SPELL_CAST, when_cast_spell),
            lose_1_life,
            "Whenever you cast a spell, you lose 1 life.",
        ),
        triggered(
            when_asked(
                EventMask::BECAME_TARGET,
                when_enchantment_becomes_target_spell_ability,
            ),
            sacrifice,
            "When this enchantment becomes the target of a spell or ability, sacrifice it.",
        ),
    ],
    ..oracle::TREACHEROUS_BLESSING
};
