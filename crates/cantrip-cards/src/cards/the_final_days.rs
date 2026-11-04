use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, flashback, history, tokens};
use cantrip_oracle as oracle;

/// The 2/2 black Horror creature token.
pub static HORROR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Horror)
};

/// "Create two tapped 2/2 black Horror creature tokens. If this spell was cast from a graveyard, instead create X of those
/// tokens, where X is the number of creature cards in your graveyard." (CR 111.1, 608.2, 614.1c): X is counted once, as the
/// spell resolves (the rulings), in its controller's graveyard - the spell itself is on the stack, so it is no creature card
/// there - and the tokens are created together (CR 101.4), tapped, whichever number it is.
fn horrors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = if history::cast_from_graveyard(g.history(), item.object).is_some() {
        creature_cards_in_graveyard(g, item.controller) as u32
    } else {
        2
    };
    tokens::create_by_rule(g, item.controller, &HORROR, count, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], horrors),
    ways: &[flashback::way(cost("{4}{B}{B}"), "Flashback {4}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)")],
    ..oracle::THE_FINAL_DAYS
};

pub static EXTRA: &[&CardDef] = &[&HORROR];
