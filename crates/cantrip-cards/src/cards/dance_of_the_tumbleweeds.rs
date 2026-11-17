use cantrip_api::*;
use cantrip_mechanics::{lands_you_control, library, select, spree, tokens};
use cantrip_oracle as oracle;

/// Mode 0: "Search your library for a basic land card or a Desert card, put it onto the battlefield, then shuffle" (CR 701.23,
/// 205.3i): one card, untapped.
fn search_for_a_basic_land_or_desert(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::BASIC_LAND_OR_DESERT_CARD, false)?;
    Ok(())
}

/// Mode 1: "Create an X/X green Elemental creature token, where X is the number of lands you control" (CR 111.4, 107.3): the lands
/// as the spell resolves, so the land mode before it (modes happen in printed order, CR 700.2d) counts. The token is X/X as it is made,
/// never a 0/0 for a moment; with no lands it is a 0/0 that state-based actions put into the graveyard (CR 704.5f).
fn create_an_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let size = lands_you_control(g, item.controller) as i32;
    let values = Copiable::of(g.card_id(&tokens::ELEMENTAL_X_X_GREEN)).sized(size, size);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(search_for_a_basic_land_or_desert, "+ {1} \u{2014} Search your library for a basic land card or a Desert card, put it onto the battlefield, then shuffle.")
        .cost(mana("{1}")),
    mode(create_an_elemental, "+ {3} \u{2014} Create an X/X green Elemental creature token, where X is the number of lands you control.").cost(mana("{3}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::DANCE_OF_THE_TUMBLEWEEDS };

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_X_X_GREEN];
