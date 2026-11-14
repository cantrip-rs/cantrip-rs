use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Locthwain Scorn.
pub static LOCTHWAIN_SCORN: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_you_gain_life::<-3, -3, 2>),
    ..oracle::LOCTHWAIN_SCORN
};

/// "At the beginning of your upkeep, put target creature card from a graveyard onto the battlefield under your control."
/// Any player's graveyard; with the card gone as it resolves the ability does nothing (CR 608.2b).
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield(card, item.controller, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_UPKEEP, reanimate, "At the beginning of your upkeep, put target creature card from a graveyard onto the battlefield under your control.")
        .targets(&[targets::CREATURE_CARD_IN_A_GRAVEYARD])],
    adventure: Some(&LOCTHWAIN_SCORN),
    ..oracle::VIRTUE_OF_PERSISTENCE
};

pub static EXTRA: &[&CardDef] = &[&LOCTHWAIN_SCORN];
