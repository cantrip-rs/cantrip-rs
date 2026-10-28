use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715), "Spew Flame deals 5 damage to target creature."
fn spew_flame(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 5);
    Ok(())
}

pub static SPEW_FLAME: CardDef = CardDef { spell: spell(&[targets::CREATURE], spew_flame), ..oracle::SPEW_FLAME };

pub static CARD: CardDef = CardDef { adventure: Some(&SPEW_FLAME), ..oracle::SMAUG_THE_GREAT_CALAMITY };

pub static EXTRA: &[&CardDef] = &[&SPEW_FLAME];
