use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Ride the Rails, "Target creature gets +2/+1 until end of turn."
pub static RIDE_THE_RAILS: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<2, 1>), ..oracle::RIDE_THE_RAILS };

pub static CARD: CardDef = CardDef { adventure: Some(&RIDE_THE_RAILS), ..oracle::MINECART_DAREDEVIL };

pub static EXTRA: &[&CardDef] = &[&RIDE_THE_RAILS];
