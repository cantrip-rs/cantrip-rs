use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create two tapped 1/1 black Bat creature tokens with flying." (CR 111.1, 111.5): both tokens are created together, tapped,
/// under the activating player's control.
fn create_two_tapped_bats(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let bat = g.card_id(&tokens::BAT);
    g.create_tokens(bat, item.controller, 2, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}{B}").and(EXILE_THIS),
        create_two_tapped_bats,
        "{2}{B}{B}, Exile this card from your graveyard: Create two tapped 1/1 black Bat creature tokens with flying.",
    )
    .from(ZoneKind::Graveyard)],
    ..oracle::LEERING_ONLOOKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::BAT];
