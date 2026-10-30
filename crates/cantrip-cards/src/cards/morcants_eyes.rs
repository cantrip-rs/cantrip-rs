use cantrip_api::*;
use cantrip_mechanics::{effects, subtype_cards_in_graveyard, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create X 2/2 black and green Elf creature tokens, where X is the number of Elf cards in your graveyard": counted once, as the
/// ability resolves, and the Enchantment (a Kindred Elf card) is in the graveyard by then, having been sacrificed as a cost, so
/// it counts itself.
fn create_elves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let x = subtype_cards_in_graveyard(g, item.controller, elf);
    tokens::create(g, item.controller, &tokens::ELF_BLACK_GREEN, x as u32)?;
    Ok(())
}

/// The Elf token the ability makes.
pub static EXTRA: &[&CardDef] = &[&tokens::ELF_BLACK_GREEN];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, |g, item| effects::surveil(g, item, 1), "At the beginning of your upkeep, surveil 1."),
        ability(
            cost("{4}{G}{G}").and(SACRIFICE_THIS),
            create_elves,
            "{4}{G}{G}, Sacrifice this enchantment: Create X 2/2 black and green Elf creature tokens, where X is the number of Elf cards in your graveyard. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::MORCANTS_EYES
};
