use cantrip_api::*;
use cantrip_mechanics::tokens::{self, KITHKIN};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&KITHKIN];

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{W}").and(EXILE_THIS),
        tokens::create_a_kithkin,
        "{W}, Exile this card from your graveyard: Create a 1/1 green and white Kithkin creature token. Activate only as a sorcery.",
    )
    .from(ZoneKind::Graveyard)
    .sorcery_speed()],
    ..oracle::GOLDMEADOW_NOMAD
};
