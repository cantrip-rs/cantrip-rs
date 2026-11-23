use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "As Mimeoplasm enters, exile up to X creature cards from your graveyard. It enters with three +1/+1 counters on it for each
/// creature card exiled this way" (CR 614.12, 614.1c, 122.6): the cards are linked to the permanent the spell becomes
/// (CR 607.2a, 607.2q), so its own ability finds them.
fn as_enters_exile_x_creature_cards(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let asker = Asker::new(entering.object, entering.controller);
    let cards = g.state().zone(Zone::graveyard(entering.controller)).iter().rev().copied()
        .filter(|&id| targets::card_in_your_graveyard(g, &asker, Target::Object(id), &[]))
        .collect();
    let selected = choose::select(
        g,
        entering.controller,
        entering.object,
        Purpose::Exile,
        cards,
        select::creature_cards(select::up_to(entering.x)),
    )?;
    let exiled = g.exile_with(&selected, g.text_linked(entering.object), false);
    entering.counters[CounterKind::PlusOne as usize] += 3 * exiled.len() as i32;
    Ok(())
}

/// "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability" (CR 707.9b,
/// 707.2): the card's copiable values with the base size 0/0, and this ability added so a later copy has it. It states no
/// duration (CR 611.2a), and the +1/+1 counters stay.
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(card).sized(0, 0).with_ability(item.ability);
    g.become_copy(item.object, item.object, values, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(as_enters_exile_x_creature_cards),
    abilities: &[ability(
        cost("{2}"),
        become_copy,
        "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability.",
    )
    .targets(&[targets::CREATURE_CARD_EXILED_WITH_THIS])],
    ..oracle::MIMEOPLASM_REVERED_ONE
};
