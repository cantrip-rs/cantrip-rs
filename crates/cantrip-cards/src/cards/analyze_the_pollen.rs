use cantrip_api::*;
use cantrip_mechanics::library::{search_to_hand, Find};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, you may collect evidence 8" (CR 701.59a, 601.2b): announced like any optional
/// additional cost; the spell has `paid` set when it was paid. It cannot be chosen without cards in the graveyard with total
/// mana value 8 or more (CR 701.59a ruling: "you can't choose to collect evidence at all").
const OPTIONAL_COST: OptionalCost = OptionalCost::new(Cost { collect_evidence: 8, ..Cost::NONE });

const LABELS: &[&str] = &["Cast without collecting evidence", "Collect evidence 8"];

/// "Search your library for a basic land card. If evidence was collected, instead search your library for a creature or
/// land card. Reveal that card, put it into your hand, then shuffle." Which kind of card may be found is decided by whether
/// the additional cost was paid; the card found is revealed (CR 701.23), and the library is shuffled even if nothing is
/// found.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let evidence = item.paid > 0;
    search_to_hand(g, item.controller, item.object, Find::ONE.revealed(), |q, c| {
        let chars = q.chars(c);
        if evidence {
            chars.is(CardType::Creature) || chars.is(CardType::Land)
        } else {
            chars.is(CardType::Land) && chars.supertypes.basic
        }
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    optional_cost: Some(OPTIONAL_COST),
    labels: LABELS,
    ..oracle::ANALYZE_THE_POLLEN
};
