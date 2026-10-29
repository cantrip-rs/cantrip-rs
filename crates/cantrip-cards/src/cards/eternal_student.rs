use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create two 1/1 white and black Inkling creature tokens with flying", together (CR 111.1, 101.4).
fn inklings(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::INKLING, 2)?;
    Ok(())
}

/// "{1}{B}, Exile this card from your graveyard": an activated ability that functions only in its owner's graveyard
/// (CR 113.6k), `from` says so; exiling the card is a cost (CR 602.2), so the tokens come even if the ability is countered
/// or the card is gone by the time it resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B}").and(EXILE_THIS),
        inklings,
        "{1}{B}, Exile this card from your graveyard: Create two 1/1 white and black Inkling creature tokens with flying.",
    )
    .from(ZoneKind::Graveyard)],
    ..oracle::ETERNAL_STUDENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::INKLING];
