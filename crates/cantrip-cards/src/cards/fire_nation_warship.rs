use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// Reach comes from the printed keywords. "When this Vehicle dies" is a leaves-the-battlefield trigger (CR 603.10a). "Dies" is
/// "is put into a graveyard from the battlefield" (CR 700.4), of any permanent, so a Vehicle that was never crewed (only an
/// artifact) dies too when it is destroyed. Creating a Clue is not investigating (the 2025-10-02 ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_DIES,
            |g, item| tokens::create(g, item.controller, &tokens::CLUE, 1).map(|_| ()),
            "When this Vehicle dies, create a Clue token. (It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::FIRE_NATION_WARSHIP
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
