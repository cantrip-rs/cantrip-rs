use cantrip_api::*;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 white Human creature token."
fn create_a_human(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HUMAN, 1)?;
    Ok(())
}

/// "Choose two" (CR 700.2): the modes are carried out in the order printed (CR 700.2d), and the same mode cannot be chosen twice.
static MODAL: ModalDef = modes(&[
    mode(
        |g, item| lands::search_basic_land_to_battlefield(g, item.controller, item.object, true),
        "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    ),
    mode(create_a_human, "Create a 1/1 white Human creature token."),
    mode(tokens::create_a_food_token, "Create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")"),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::RETURN_FROM_THE_WILDS };

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN, &tokens::FOOD];
