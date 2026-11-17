use cantrip_api::*;
use cantrip_mechanics::{effects, hand, library, select, spree};
use cantrip_oracle as oracle;

/// Mode 0: "Mill four cards. You may put up to two creature and/or land cards from among the milled cards into your hand." (CR 701.17a,
/// 608.2d) The cards the mill moved are the ones offered; nothing is asked when none was milled.
fn mill_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_up_to_two_to_hand(g, item, 4, Selection::of(select::up_to(2), &[select::CREATURE_OR_LAND]).called("creature or land"))
}

/// Mode 1: "You may put up to two creature cards from your hand onto the battlefield." (CR 608.2d, 101.4) A card put this way is not cast
/// and not played. When the first mode was chosen too, it is resolved first (CR 700.2a), so a card it put into the hand is available.
fn put_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_up_to_onto_battlefield(g, item.controller, item.object, select::creature_cards(select::up_to(2)))?;
    Ok(())
}

/// Mode 2: "Creatures you control with power 4 or greater gain hexproof and indestructible until end of turn." (CR 611.2c) The creatures
/// as the mode resolves, their power read then (so after the creatures the second mode put onto the battlefield); none that enters or
/// grows later gains anything.
fn guard_big_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_where_gain(g, item, &[Keyword::Hexproof, Keyword::Indestructible], |g, id| g.chars(id).power.is_some_and(|p| p >= 4));
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(mill_four, "+ {2} \u{2014} Mill four cards. You may put up to two creature and/or land cards from among the milled cards into your hand.").cost(mana("{2}")),
    mode(put_creatures, "+ {4}{G} \u{2014} You may put up to two creature cards from your hand onto the battlefield.").cost(mana("{4}{G}")),
    mode(guard_big_creatures, "+ {1} \u{2014} Creatures you control with power 4 or greater gain hexproof and indestructible until end of turn.").cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SMUGGLERS_SURPRISE };
