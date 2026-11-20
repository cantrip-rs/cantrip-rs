use cantrip_api::*;
use cantrip_oracle as oracle;

/// Plant Beans: "You may play an additional land this turn." (CR 305.2): until end of turn, and cumulative with other effects
/// that do the same.
fn play_an_additional_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.additional_land_plays(item, 1);
    Ok(())
}

/// The Adventure (CR 715): Plant Beans, "You may play an additional land this turn."
pub static PLANT_BEANS: CardDef = CardDef { spell: spell(&[], play_an_additional_land), ..oracle::PLANT_BEANS };

pub static CARD: CardDef = CardDef { adventure: Some(&PLANT_BEANS), ..oracle::BEANSTALK_WURM };

pub static EXTRA: &[&CardDef] = &[&PLANT_BEANS];
