use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{self, THIS_ATTACKS};
use cantrip_oracle as oracle;

/// Prowess (CR 702.108), and "Whenever Queen Brahne attacks, create a 0/1 black Wizard creature token with 'Whenever you cast a
/// noncreature spell, this token deals 1 damage to each opponent.'" (CR 508.1m, 603.2, 111.1): the token enters untapped and is
/// not attacking.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            THIS_ATTACKS,
            tokens::create_a_wizard,
            "Whenever Queen Brahne attacks, create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"",
        ),
    ],
    ..oracle::QUEEN_BRAHNE
};

pub static EXTRA: &[&CardDef] = &[&tokens::WIZARD];
