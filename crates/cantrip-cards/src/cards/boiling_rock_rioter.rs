use cantrip_api::*;
use cantrip_mechanics::{choose, firebending, lands, targets, triggers, untapped_subtype_you_control};
use cantrip_oracle as oracle;

/// "Exile target card from a graveyard": it is exiled with this creature (CR 607.2a), for its attack trigger.
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_with(&[card], g.linked(item), false);
    Ok(())
}

/// "You may cast an Ally spell from among cards you own exiled with this creature" (CR 608.2g): only the cards its controller
/// owns count, and a card owned by an opponent is left in exile. The spell is cast for its mana cost, and whether it is an Ally
/// spell is judged of the spell it would be (CR 601.3e): the face cast (CR 709.3a, 715.3a), with its characteristic-defining
/// abilities (changeling, CR 702.73a) and the effects that would apply to it on the stack. Nothing is asked when no card there
/// can be cast.
fn cast_an_ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let owned: Vec<ObjectId> = g.exiled_with(g.linked(item)).into_iter().filter(|&card| g.obj(card).owner == you).collect();
    let ally = g.words(item.object).subtype(Subtype::Ally);
    choose::cast_up_to_among(g, you, item.object, Purpose::Cast, &owned, 1, CastTerms::default(), |g, card, w| g.chars_as_cast(card, w.way, you).has_subtype(ally))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<1>("Firebending 1 (Whenever this creature attacks, add {R}. This mana lasts until end of combat.)"),
        ability(
            tap_untapped(1, untapped_subtype_you_control::<{ lands::sub(Subtype::Ally) }>),
            exile_it,
            "Tap an untapped Ally you control: Exile target card from a graveyard.",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD]),
        triggered(
            triggers::THIS_ATTACKS,
            cast_an_ally,
            "Whenever this creature attacks, you may cast an Ally spell from among cards you own exiled with this creature.",
        ),
    ],
    ..oracle::BOILING_ROCK_RIOTER
};
