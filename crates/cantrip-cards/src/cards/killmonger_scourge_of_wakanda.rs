use cantrip_api::*;
use cantrip_mechanics::statics::{itself_with_two_creature_cards_in_your_graveyard, plus_pt};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{destroy_target, effects, select, targets};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature. When you do, destroy target nonland permanent an opponent controls." (CR 701.21a, 603.12):
/// the creature is chosen as the ability resolves, not targeted, and declining is the empty answer to an up-to-one choice. The
/// reflexive ability exists only for a creature actually sacrificed, whether or not Killmonger is still on the battlefield.
fn sacrifice_another_to_destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_sacrifice_when_you_do(g, item, select::another_creature_permanents(select::up_to(1)))
}

fn destroy_the_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        sacrifice_another_to_destroy,
        "When Killmonger enters, you may sacrifice another creature. When you do, destroy target nonland permanent an opponent controls.",
    )],
    delayed: &[delayed_trigger(destroy_the_target, "When you do, destroy target nonland permanent an opponent controls.")
        .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS])],
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_two_creature_cards_in_your_graveyard,
        plus_pt::<2, 1>,
        "As long as there are two or more creature cards in your graveyard, Killmonger gets +2/+1.",
    )],
    ..oracle::KILLMONGER_SCOURGE_OF_WAKANDA
};
