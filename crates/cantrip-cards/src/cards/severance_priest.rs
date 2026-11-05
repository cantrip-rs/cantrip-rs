use cantrip_api::*;
use cantrip_mechanics::{effects, endure, hand, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You may choose a nonland card from it. If you do, exile that card." Revealing is part of
/// the ability, so the choice is asked of any hand that is not empty, even with nothing eligible (CR 701.20a). The card is
/// remembered as exiled with this creature (CR 607.2a) and stays exiled: it is not tied to the creature remaining on the battlefield.
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    g.reveal(opponent, &hand);
    if let Some(card) = hand::look_and_may_choose(g, item, opponent, Purpose::Exile, select::NONLAND_CARD)? {
        g.exile_with(&[card], g.linked(item), false);
    }
    Ok(())
}

/// "The exiled card's owner creates an X/X white Spirit creature token, where X is the mana value of the exiled card." The token
/// is the white Spirit endure makes.
fn spirit_for_owner(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::exiled_cards_owner_creates_x_x(g, item, &endure::SPIRIT)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, reveal_and_exile, "When this creature enters, target opponent reveals their hand. You may choose a nonland card from it. If you do, exile that card.")
            .targets(&[targets::OPPONENT]),
        triggered(triggers::THIS_LEAVES, spirit_for_owner, "When this creature leaves the battlefield, the exiled card's owner creates an X/X white Spirit creature token, where X is the mana value of the exiled card."),
    ],
    ..oracle::SEVERANCE_PRIEST
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
