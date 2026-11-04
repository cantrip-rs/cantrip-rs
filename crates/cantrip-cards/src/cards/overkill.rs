use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

// "Target creature gets -0/-9999 until end of turn" (CR 611.2a, 704.5f): a toughness of 0 or less is put into its owner's graveyard
// by state-based actions, indestructible or not, and no regeneration or damage-prevention helps.
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<0, -9999>), ..oracle::OVERKILL };
