use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// The 1/1 white and black Inkling creature token with flying.
pub static INKLING: CardDef = CardDef {
    name: "Inkling Token",
    token_name: Some(&[Subtype::Inkling]),
    has_cost: false,
    colors: Some(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Black).0)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Inkling]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};

/// Target player creates the Inkling. Then, counting with it (CR 608.2c), if an opponent controls more creatures
/// than you, this creature becomes prepared, if it is still this creature on the battlefield.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.target(0).player() else { return Ok(()) };
    tokens::create(g, player, &INKLING, 1)?;
    let mine = cantrip_mechanics::creatures_you_control(g, item.controller);
    let outnumbered = g.state().opponents(item.controller).collect::<Vec<_>>().into_iter().any(|o| cantrip_mechanics::creatures_you_control(g, o) > mine);
    if outnumbered {
        g.become_prepared(item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, enters, "When this creature enters, target player creates a 1/1 white and black Inkling creature token with flying. Then if an opponent controls more creatures than you, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)")
        .targets(&[targets::PLAYER])],
    prepare: Some(&crate::swords_to_plowshares::CARD),
    ..oracle::EMERITUS_OF_TRUCE
};

pub static EXTRA: &[&CardDef] = &[&INKLING];
