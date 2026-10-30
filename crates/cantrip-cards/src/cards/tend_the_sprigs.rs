use cantrip_api::*;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Then if you control seven or more
/// lands and/or Treefolk, create a 3/4 green Treefolk creature token with reach." (CR 701.23, 608.2c): the count is made after the
/// land has come in, so it counts that land; a permanent that is both a land and a Treefolk counts once. Treefolk is a word of this
/// card's rules text (CR 612), and a permanent is one when the layers made it that type.
fn tend_the_sprigs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)?;
    let treefolk = g.words(item.object).subtype(Subtype::Treefolk);
    let count = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Land) || chars.subtypes.has(treefolk)
        })
        .count();
    if count >= 7 {
        tokens::create(g, item.controller, &tokens::TREEFOLK, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], tend_the_sprigs), ..oracle::TEND_THE_SPRIGS };

pub static EXTRA: &[&CardDef] = &[&tokens::TREEFOLK];
