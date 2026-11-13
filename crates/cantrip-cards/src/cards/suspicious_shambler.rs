use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "{4}{B}{B}, Exile this card from your graveyard: Create two 2/2 black Zombie creature tokens. Activate only as a sorcery." An
/// activated ability that functions only in its owner's graveyard (CR 113.6m), `from` says so; exiling the card is a cost
/// (CR 602.2), so the card stays in exile even if the ability is countered (CR 701.6a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{B}{B}").and(EXILE_THIS),
        tokens::create_zombies::<2>,
        "{4}{B}{B}, Exile this card from your graveyard: Create two 2/2 black Zombie creature tokens. Activate only as a sorcery.",
    )
    .from(ZoneKind::Graveyard)
    .sorcery_speed()],
    ..oracle::SUSPICIOUS_SHAMBLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];
