use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, tokens};
use cantrip_oracle as oracle;

/// "Create a tapped 2/1 black Villain creature token with menace." (CR 111.1, 614.1c).
fn tapped_villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1, true)?;
    Ok(())
}

/// "Activate only if there are two or more creature cards in your graveyard" (CR 602.5b): checked as the ability is
/// activated, in its controller's graveyard; tokens are not cards.
fn two_or_more_creature_cards_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    creature_cards_in_graveyard(q, asker.controller) >= 2
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}, {T}"),
        tapped_villain,
        "{3}, {T}: Create a tapped 2/1 black Villain creature token with menace. Activate only if there are two or more creature cards in your graveyard.",
    )
    .condition(two_or_more_creature_cards_in_your_graveyard)],
    ..oracle::ARNIM_ZOLA_BIO_FANATIC
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
