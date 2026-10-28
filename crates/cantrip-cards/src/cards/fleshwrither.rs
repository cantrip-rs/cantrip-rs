use cantrip_api::*;
use cantrip_mechanics::transfigure::transfigure;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[transfigure(
        mana("{1}{B}{B}"),
        "Transfigure {1}{B}{B} ({1}{B}{B}, Sacrifice this creature: Search your library for a creature card with the same mana value as this creature, put that card onto the battlefield, then shuffle. Transfigure only as a sorcery.)",
    )],
    ..oracle::FLESHWRITHER
};
