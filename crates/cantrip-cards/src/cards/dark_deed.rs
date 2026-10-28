use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -4/-4 until end of turn." (CR 611.2a): nothing if the creature is no longer a legal target as the
/// spell resolves (CR 608.2b); a creature it leaves with 0 toughness or less is put into its owner's graveyard by the next
/// state-based action check (CR 704.5f).
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<-4, -4>), ..oracle::DARK_DEED };
