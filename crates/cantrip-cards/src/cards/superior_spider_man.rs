use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "CopyAsEnters",
    "enter as a copy of a creature card in a graveyard?",
    "When you do, that card is exiled. If you don't, it enters as itself",
    "Choose a card",
    "Enter as itself",
);

/// Every creature card in every graveyard (CR 608.2d): chosen rather than targeted, so an opponent's graveyard is as available
/// as your own.
fn copyable(g: &dyn Game) -> Vec<ObjectId> {
    g.state()
        .seats()
        .flat_map(|p| g.state().zone(Zone::graveyard(p)).iter().copied())
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect()
}

/// Mind Swap: "You may have Superior Spider-Man enter as a copy of any creature card in a graveyard, except his name is Superior
/// Spider-Man and he's a 4/4 Spider Human Hero in addition to his other types. When you do, exile that card." A copy as he
/// enters (CR 614.1c, 707.1); the exceptions are part of the copy's own copiable values (CR 707.9b), and the stated power and
/// toughness mean a defining ability of the copied card is not copied (CR 707.9d). Nothing is asked when no graveyard has a
/// creature card. "When you do" is a reflexive trigger (CR 603.12) that exile that card.
fn mind_swap(g: &mut dyn Game, entry: &mut Entering) -> Result<(), Suspend> {
    let entering = entry.object;
    let controller = entry.controller;
    let cards = copyable(g);
    let Some(chosen) = copy::choose_copy_of_one_of(g, entry, &cards, &ASK)? else { return Ok(()) };
    let words = g.words(entering);
    entry.copy = Some(
        g.copiable_values(chosen)
            .named("Superior Spider-Man")
            .sized(4, 4)
            .with_subtype(words.subtype(Subtype::Spider))
            .with_subtype(words.subtype(Subtype::Human))
            .with_subtype(words.subtype(Subtype::Hero)),
    );
    let me = g.card_id(&CARD);
    g.reflexive_trigger(entering, controller, AbilityRef::delayed(me, 0), Target::Object(chosen));
    Ok(())
}

/// When you do, exile that card: nothing if it has left the graveyard by now, being a new object (CR 400.7).
fn exile_that_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(card) = item.subject
        && is_live(g, card)
        && g.obj(card).zone.is(ZoneKind::Graveyard)
    {
        g.exile(card);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(mind_swap),
    delayed: &[delayed_trigger(exile_that_card, "When you do, exile that card.")],
    ..oracle::SUPERIOR_SPIDER_MAN
};
