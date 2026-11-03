use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Companion — Your starting deck contains only cards with mana value 3 or greater and land cards. (If this card is your chosen companion, you may put it into your hand from outside the game for {3} as a sorcery.) When Keruga enters, draw a card for each other permanent you control with mana value 3 or greater." (CR 608.2).
fn companion_starting_deck_contains_only_companion(
    q: &dyn Query,
    deck: &StartingDeck,
) -> bool {
    deck.cards
        .iter()
        .all(|&id| (i64::from(q.mana_value(id))) >= 3 || q.chars(id).is(CardType::Land))
}

/// "When Keruga enters" (CR 603.2).
fn when_keruga_enters(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When Keruga enters, draw a card for each other permanent you control with mana value 3 or greater." (CR 608.2, 121.1).
fn draw_card_other_permanent_control(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = {
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(item.subject.object() == Some(id))
                    && item.controller == q.controller(id)
                    && (i64::from(q.mana_value(id))) >= 3
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as u32
    };
    g.draw(item.controller, amount_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_keruga_enters),
            draw_card_other_permanent_control,
            "When Keruga enters, draw a card for each other permanent you control with mana value 3 or greater.",
        ),
    ],
    companion: Some(companion_starting_deck_contains_only_companion),
    ..oracle::KERUGA_THE_MACROSAGE
};
