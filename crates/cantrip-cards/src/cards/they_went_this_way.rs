use cantrip_api::*;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Investigate." (CR 701.23, 701.16a):
/// two sentences, so the investigate is a separate instruction (CR 608.2c) done even when the search finds nothing.
fn search_then_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)?;
    g.end_sentence();
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], search_then_investigate), ..oracle::THEY_WENT_THIS_WAY };

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
