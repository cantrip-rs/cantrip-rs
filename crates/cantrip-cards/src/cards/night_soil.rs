use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "{1}, Exile two creature cards from a single graveyard: Create a 1/1 green Saproling creature token." The two cards are
/// of one graveyard, any player's, chosen and exiled as the cost is paid (CR 601.2h by way of 602.2b), so no one can
/// respond to their exile; with no graveyard holding two creature cards the cost can't be paid (CR 118.3; the 2008-10-01
/// ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(exile_from_a_graveyard_of(2, filters::creature)),
        tokens::create_a_saproling,
        "{1}, Exile two creature cards from a single graveyard: Create a 1/1 green Saproling creature token.",
    )],
    ..oracle::NIGHT_SOIL
};

pub static EXTRA: &[&CardDef] = &[&tokens::SAPROLING];
