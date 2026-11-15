use cantrip_api::*;
use cantrip_mechanics::{effects, haste_until_end_of_turn, targets};
use cantrip_oracle as oracle;

/// Entering: "Put a creature card from a graveyard onto the battlefield under your control. It gains haste until end of
/// turn." The card is chosen as the spell resolves, not targeted (the rulings), from every graveyard, so a creature just
/// milled by a fused Breaking may be chosen (CR 702.102d, 608.2d); it is not optional, so one is chosen whenever any
/// graveyard holds a creature card.
fn entering(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(entered) = effects::put_a_creature_card_from_a_graveyard_onto_the_battlefield(g, item)? {
        haste_until_end_of_turn(g, item.object, entered);
    }
    Ok(())
}

pub static BREAKING: CardDef = CardDef { spell: spell(&[targets::PLAYER], effects::target_player_mills::<8>), ..oracle::BREAKING };

pub static ENTERING: CardDef = CardDef { spell: spell(&[], entering), ..oracle::ENTERING };

/// "Fuse (You may cast one or both halves of this card from your hand.)" (CR 702.102): the core makes the fused split
/// spell from the halves.
pub static CARD: CardDef = CardDef { split: Some((&BREAKING, &ENTERING)), fuse: true, ..oracle::BREAKING_ENTERING };

pub static EXTRA: &[&CardDef] = &[&BREAKING, &ENTERING];
