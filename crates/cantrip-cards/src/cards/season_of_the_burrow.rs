use cantrip_api::*;
use cantrip_mechanics::tokens::{self, RABBIT};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// {P} - "Create a 1/1 white Rabbit creature token." (CR 111.1).
fn create_a_rabbit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &RABBIT, 1)?;
    Ok(())
}

/// {P}{P} - "Exile target nonland permanent. Its controller draws a card." (CR 701.13a, 121.1): the controller is read before the
/// exile, as the permanent was, and draws whether or not the exile happened to move it (a separate instruction, CR 608.2c). With
/// the target gone the mode does nothing (CR 608.2b), so nobody draws.
fn exile_target_nonland_permanent_its_controller_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(permanent);
    g.exile(permanent);
    g.draw(controller, 1);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(create_a_rabbit, "{P} \u{2014} Create a 1/1 white Rabbit creature token.").pawprints(1),
    mode(exile_target_nonland_permanent_its_controller_draws, "{P}{P} \u{2014} Exile target nonland permanent. Its controller draws a card.")
        .targets(&[targets::NONLAND_PERMANENT])
        .pawprints(2),
    // The counter is there as it enters (CR 614.1c, 122.6); with the card gone from the graveyard nothing happens (CR 608.2b).
    mode(
        effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::Indestructible as usize }, 1>,
        "{P}{P}{P} \u{2014} Return target permanent card with mana value 3 or less from your graveyard to the battlefield with an indestructible counter on it.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD_WITH_MANA_VALUE_3_OR_LESS])
    .pawprints(3),
])
.pawprints(5);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::SEASON_OF_THE_BURROW };

pub static EXTRA: &[&CardDef] = &[&RABBIT];
