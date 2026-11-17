use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{choose, effects, graveyards, select, triggers};
use cantrip_oracle as oracle;

const BECOME_COPY: Question = Question::new("BecomeCopy", "have Lazav become a copy of the exiled creature card?", "Until end of turn", "Become a copy", "Don't");

/// "Put a +1/+1 counter on Lazav. Then you may exile a card from a graveyard. If a creature card was exiled this way, you may have
/// Lazav become a copy of that card until end of turn." (CR 122.1, 608.2d, 707.2, 611.2): the card is chosen as the ability resolves,
/// from any graveyard, not targeted. It is judged a creature card where it was, as exiled (CR 400.7), and only if the exile
/// happened. Lazav becomes a copy of what was printed on the card (the exiled card's copiable values), keeping its counter, and
/// is neither entering nor leaving, so nothing triggers (CR 707.9, the rulings). With Lazav gone nothing is copied.
fn counter_then_exile_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    let you = item.controller;
    let cards: Vec<ObjectId> = graveyards::cards_in_graveyards(g.state()).collect();
    let Some(&card) = choose::select(g, you, item.object, Purpose::Exile, cards, select::cards(select::up_to(1)))?.first() else {
        return Ok(());
    };
    let creature = g.chars(card).is(CardType::Creature);
    let exiled = g.exile(card);
    if creature && g.obj(exiled).zone.is(ZoneKind::Exile) && g.is_on_battlefield(item.object) && ask_may(g, you, item.object, Purpose::Ask(&BECOME_COPY))? {
        let values = g.copiable_values(exiled);
        g.become_copy(item.object, item.object, values, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_COMMIT_A_CRIME,
        counter_then_exile_and_copy,
        "Whenever you commit a crime, put a +1/+1 counter on Lazav. Then you may exile a card from a graveyard. If a creature card was exiled this way, you may have Lazav become a copy of that card until end of turn. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )
    .triggers_each_turn(1)],
    ..oracle::LAZAV_FAMILIAR_STRANGER
};
