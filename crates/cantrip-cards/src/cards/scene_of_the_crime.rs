use cantrip_api::*;
use cantrip_mechanics::{effects, mana::enters_tapped, TAP_AN_UNTAPPED_CREATURE};
use cantrip_oracle as oracle;

/// An Artifact Land — Clue, which as a Clue has no ability of its own (CR 205.3g): everything it does is printed. "{T}, Tap an
/// untapped creature you control: Add one mana of any color." is a mana ability (CR 605.1a) whose creature is chosen with the
/// activation. "{2}, Sacrifice this land: Draw a card." (CR 701.21a): sacrificing it is part of the cost.
pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped),
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP.and(TAP_AN_UNTAPPED_CREATURE), add_any_color(1), "{T}, Tap an untapped creature you control: Add one mana of any color."),
        ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this land: Draw a card."),
    ],
    ..oracle::SCENE_OF_THE_CRIME
};
