use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, mill three cards" (CR 701.17).
fn mill_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 3);
    Ok(())
}

/// "Target creature or Spacecraft card from your graveyard" (CR 115.1, 205.3g): a card in the asker's own graveyard with the
/// creature type or the Spacecraft subtype.
fn creature_or_spacecraft_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_card_or_subtype_card_in_your_graveyard(q, asker, t, Subtype::Spacecraft)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, mill_three, "When this creature enters, mill three cards."),
        ability(
            cost("{3}{B}{G}").and(SACRIFICE_THIS),
            effects::return_first_target_to_battlefield,
            "{3}{B}{G}, Sacrifice this creature: Return target creature or Spacecraft card from your graveyard to the battlefield. Activate only as a sorcery.",
        )
        .sorcery_speed()
        .targets(&[target(creature_or_spacecraft_card_in_your_graveyard)]),
    ],
    ..oracle::SEEDSHIP_BROODTENDER
};
