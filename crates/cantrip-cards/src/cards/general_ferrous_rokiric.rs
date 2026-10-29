use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 4/4 red and white Golem artifact creature token." Its colors and creature type are the words of the text
/// that makes it (CR 612.4).
fn make_golem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GOLEM, 1)?;
    Ok(())
}

/// The 4/4 red and white Golem artifact creature token.
pub static GOLEM: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::White).0)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..token!(Golem)
};

pub static CARD: CardDef = CardDef {
    // "Hexproof from monocolored" (CR 702.11d).
    hexproof_from: HexproofFrom::MONOCOLORED,
    abilities: &[triggered(
        triggers::MULTICOLORED_SPELL_YOU_CAST,
        make_golem,
        "Whenever you cast a multicolored spell, create a 4/4 red and white Golem artifact creature token.",
    )],
    ..oracle::GENERAL_FERROUS_ROKIRIC
};

pub static EXTRA: &[&CardDef] = &[&GOLEM];
