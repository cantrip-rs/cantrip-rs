use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_mechanics::{choose, lands, select, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Mill four cards, then put up to two land cards from among them into your hand. (Then exile this card.
/// You may cast the creature later from exile.)" The cards milled (CR 701.17a) are the ones to choose from, as they are where they
/// went (a graveyard, or exile by a replacement effect); the player chooses up to two of the land cards among them, none if there
/// are none, and nothing is asked when none were milled. The exile is the core's (CR 715.4), done only if the spell resolves.
fn mill_four_then_up_to_two_lands_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let milled = g.mill(you, 4);
    g.end_sentence();
    let lands = choose::select(g, you, item.object, Purpose::PutIntoHand, milled, select::land_cards(select::up_to(2)))?;
    g.return_to_hand(&lands);
    Ok(())
}

pub static SILVAN_RALLY: CardDef = CardDef { spell: spell(&[], mill_four_then_up_to_two_lands_to_hand), ..oracle::SILVAN_RALLY };

pub static CARD: CardDef = CardDef {
    // "Other Elves you control get +1/+1" (CR 613.4c): every other permanent that is an Elf as the layers make it, the type read
    // through the words of this card's text (CR 612); the tokens it makes among them.
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        plus_pt::<1, 1>,
        "Other Elves you control get +1/+1.",
    )],
    abilities: &[triggered(
        LANDFALL,
        tokens::create_an_elf,
        "Landfall \u{2014} Whenever a land you control enters, create a 1/1 green Elf creature token.",
    )],
    adventure: Some(&SILVAN_RALLY),
    ..oracle::THRANDUIL_SINDARIN_LIEGE
};

pub static EXTRA: &[&CardDef] = &[&SILVAN_RALLY, &tokens::ELF];
