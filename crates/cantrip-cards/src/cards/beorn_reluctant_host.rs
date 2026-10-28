use cantrip_api::*;
use cantrip_oracle as oracle;

/// Till and Tend: "You may play an additional land this turn." (CR 305.2, 611.2c): until end of turn, and cumulative with other
/// effects that do the same (the ruling).
fn play_an_additional_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.additional_land_plays(item, 1);
    Ok(())
}

/// The Adventure (CR 715): Till and Tend.
pub static TILL_AND_TEND: CardDef = CardDef { spell: spell(&[], play_an_additional_land), ..oracle::TILL_AND_TEND };

pub static CARD: CardDef = CardDef { adventure: Some(&TILL_AND_TEND), ..oracle::BEORN_RELUCTANT_HOST };

pub static EXTRA: &[&CardDef] = &[&TILL_AND_TEND];
