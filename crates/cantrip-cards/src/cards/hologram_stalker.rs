use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 3/3 colorless Illusion artifact creature token." (CR 701.7, 111.2).
fn illusion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ILLUSION_3_3_ARTIFACT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return target permanent card from your graveyard to your hand.")
            .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD]),
        triggered(triggers::CARDS_LEAVE_YOUR_GRAVEYARD, illusion, "Whenever a card leaves your graveyard, create a 3/3 colorless Illusion artifact creature token. This ability triggers only once each turn.")
            .triggers_each_turn(1),
    ],
    ..oracle::HOLOGRAM_STALKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ILLUSION_3_3_ARTIFACT];
