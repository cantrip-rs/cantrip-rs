use cantrip_api::*;
use cantrip_mechanics::{library::look_at_top_take_one_rest_to_graveyard, tokens};
use cantrip_oracle as oracle;

/// "Create two 3/3 blue and red Elemental creature tokens with flying." (CR 111.1, 701.7)
fn elementals(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELEMENTAL_3_3_BLUE_RED, 2)?;
    Ok(())
}

/// "{2}, Discard this card: Look at the top two cards of your library. Put one of them into your hand and the other into your
/// graveyard." (CR 701.20a, 400.3): the card is discarded as a cost, so it is already in the graveyard when this resolves.
fn dance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 2)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], elementals),
    abilities: &[ability(
        cost("{2}").and(DISCARD_THIS),
        dance,
        "{2}, Discard this card: Look at the top two cards of your library. Put one of them into your hand and the other into your graveyard.",
    )
    .from(ZoneKind::Hand)],
    ..oracle::VISIONARYS_DANCE
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_3_3_BLUE_RED];
