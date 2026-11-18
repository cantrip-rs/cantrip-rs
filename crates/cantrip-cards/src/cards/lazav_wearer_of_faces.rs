use cantrip_api::*;
use cantrip_mechanics::tokens::investigate;
use cantrip_mechanics::triggers::{self, THIS_ATTACKS};
use cantrip_mechanics::{choose, lands, select, targets};
use cantrip_oracle as oracle;

/// "Whenever Lazav attacks, exile target card from a graveyard, then investigate." (CR 607.2a, 608.2b, 701.16): the card is
/// remembered as exiled with this Lazav (the second ability asks for it). With its one target gone the ability is removed from
/// the stack without resolving (CR 608.2b), so it never runs and no Clue is made either.
fn exile_then_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::legal_objects(g, item);
    g.exile_with(&cards, g.linked(item), false);
    investigate(g, item.controller)
}

/// "Whenever you sacrifice a Clue, you may have Lazav become a copy of a creature card exiled with it until end of turn."
/// (CR 707.2, 607.2a, 611.2a): chosen as the ability resolves, not targeted, among the creature cards exiled with this Lazav;
/// the "may" is the choice of none. It becomes the card's copiable values with no exception, so it keeps none of its own text.
/// It stays the same object (CR 707.4), so the cards stay exiled with it.
fn become_copy_of_exiled(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let Some(&card) = choose::select(g, item.controller, item.object, Purpose::CopyWhich, exiled, select::creature_cards(select::up_to(1)))?.first() else {
        return Ok(());
    };
    let values = g.copiable_values(card);
    g.become_copy(item.object, item.object, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            exile_then_investigate,
            "Whenever Lazav attacks, exile target card from a graveyard, then investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Clue) }>),
            become_copy_of_exiled,
            "Whenever you sacrifice a Clue, you may have Lazav become a copy of a creature card exiled with it until end of turn.",
        )
        .on_sacrifice(),
    ],
    ..oracle::LAZAV_WEARER_OF_FACES
};

pub static EXTRA: &[&CardDef] = &[&cantrip_mechanics::tokens::CLUE];
