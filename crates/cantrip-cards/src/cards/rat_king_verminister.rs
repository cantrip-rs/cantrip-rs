use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{history, same_name, targets, tokens};
use cantrip_oracle as oracle;

/// A 1/1 black Rat creature token.
pub static RAT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Rat)
};

/// A Rat, read through this card's words (CR 612): "Sacrifice three Rats".
fn a_rat(q: &dyn Query, me: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(me.source).subtype(Subtype::Rat))
}

/// "Create a 1/1 black Rat creature token and put a +1/+1 counter on Rat King" (CR 111.1, 122.1): the counter goes on Rat King
/// if it is still on the battlefield as the object it was (CR 400.7).
fn make_a_rat_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &RAT, 1)?;
    g.end_sentence();
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Return target creature card and all other cards with the same name as that card from your graveyard to the battlefield
/// tapped" (CR 400.7): together, the target and the graveyard's cards named like it (a card of several names, such as a
/// meld or double-faced card, counts for each). With the target gone the spell or ability is removed before this runs
/// (CR 608.2b).
fn return_it_and_its_namesakes(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let player = item.controller;
    let mut returning = vec![(target, player, true)];
    for &card in g.state().zone(Zone::graveyard(player)) {
        if card != target && same_name(g, target, card) {
            returning.push((card, player, true));
        }
    }
    g.put_all_onto_battlefield_as(&returning)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_END_STEP,
            make_a_rat_and_grow,
            "Disappear — At the beginning of your end step, if a permanent left the battlefield under your control this turn, create a 1/1 black Rat creature token and put a +1/+1 counter on Rat King.",
        )
        .condition(history::disappear),
        ability(
            cost("{T}").and(sacrifice(3, a_rat)),
            return_it_and_its_namesakes,
            "{T}, Sacrifice three Rats: Return target creature card and all other cards with the same name as that card from your graveyard to the battlefield tapped.",
        )
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::RAT_KING_VERMINISTER
};

pub static EXTRA: &[&CardDef] = &[&RAT];
