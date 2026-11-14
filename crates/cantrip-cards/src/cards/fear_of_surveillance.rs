use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

// Vigilance is a core keyword, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, |g, item| effects::surveil(g, item, 1), "Whenever this creature attacks, surveil 1.")],
    ..oracle::FEAR_OF_SURVEILLANCE
};
