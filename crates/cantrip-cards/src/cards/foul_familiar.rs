use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "This creature can't block." (CR 604.1).
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBlock]),
    abilities: &[
        ability(
            cost("{B}").and(life(1)),
            effects::return_this_to_hand,
            "{B}, Pay 1 life: Return this creature to its owner's hand.",
        ),
    ],
    ..oracle::FOUL_FAMILIAR
};
