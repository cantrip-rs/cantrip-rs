use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects;
use cantrip_mechanics::tokens::{PILOT, TREASURE};
use cantrip_oracle as oracle;

/// Draw a card. Then if there are no charge counters on this Vehicle, create a Treasure token and a Pilot token, as one
/// instruction. The count is read after the draw, and last known where the Vehicle has since left (CR 608.2h).
fn draw_then_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    if g.obj(item.object).counters(CounterKind::Charge) > 0 {
        return Ok(());
    }
    let treasure = g.card_id(&TREASURE);
    let pilot = g.card_id(&PILOT);
    g.create_tokens_together(&[(treasure, item.controller), (pilot, item.controller)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "This Vehicle enters with three charge counters on it" (CR 614.1c).
    enters: Some(effects::enters_with_counters::<{ CounterKind::Charge as usize }, 3>),
    abilities: &[
        ability(cost("{2}, {T}").and(remove_counters(CounterKind::Charge, 1)), draw_then_tokens, "{2}, {T}, Remove a charge counter from Reckoner Bankbuster: Draw a card. Then if there are no charge counters on Reckoner Bankbuster, create a Treasure token and a 1/1 colorless Pilot creature token with \"This token crews Vehicles as though its power were 2 greater.\""),
        crew(3, "Crew 3 (Tap any number of other untapped creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::RECKONER_BANKBUSTER
};

pub static EXTRA: &[&CardDef] = &[&TREASURE, &PILOT];
