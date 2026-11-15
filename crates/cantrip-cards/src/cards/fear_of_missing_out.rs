use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS, you_have_delirium};
use cantrip_oracle as oracle;

/// Untap target creature. After this phase, there is an additional combat phase (CR 500.8).
fn untap_and_extra_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.untap(creature);
    }
    g.add_combat_phase();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::discard_then_draw(g, item, 1), "When this creature enters, discard a card, then draw a card."),
        triggered(cantrip_mechanics::triggers::THIS_ATTACKS_FOR_THE_FIRST_TIME_EACH_TURN,untap_and_extra_combat, "Delirium \u{2014} Whenever this creature attacks for the first time each turn, if there are four or more card types among cards in your graveyard, untap target creature. After this phase, there is an additional combat phase.")
            .targets(&[targets::CREATURE])
            .condition(you_have_delirium),
    ],
    ..oracle::FEAR_OF_MISSING_OUT
};
