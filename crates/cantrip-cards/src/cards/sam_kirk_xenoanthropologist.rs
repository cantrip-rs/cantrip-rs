use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a): create a Clue token, an artifact with "{2}, Sacrifice this token: Draw a card."
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::opponent_casts_creature_spell),
        tokens::you_investigate,
        "Whenever an opponent casts a creature spell, investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
    )],
    ..oracle::SAM_KIRK_XENOANTHROPOLOGIST
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
