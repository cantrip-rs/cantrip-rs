use cantrip_api::*;
use cantrip_mechanics::{card_types_in_graveyard, effects, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Create a number of 1/1 black and green Insect creature tokens with flying equal to the number of card types among cards in your
/// graveyard" (CR 111.1, 205.2a): the types counted as the ability resolves, after its cost has put Broodspinner itself there.
fn insects_for_each_card_type_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = card_types_in_graveyard(g, item.controller) as u32;
    tokens::create(g, item.controller, &tokens::INSECT_BLACK_GREEN_FLYING, count)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 2), "When this creature enters, surveil 2."),
        ability(
            cost("{4}{B}{G}").and(TAP).and(SACRIFICE_THIS),
            insects_for_each_card_type_in_your_graveyard,
            "{4}{B}{G}, {T}, Sacrifice this creature: Create a number of 1/1 black and green Insect creature tokens with flying equal to the number of card types among cards in your graveyard.",
        ),
    ],
    ..oracle::BROODSPINNER
};

pub static EXTRA: &[&CardDef] = &[&tokens::INSECT_BLACK_GREEN_FLYING];
