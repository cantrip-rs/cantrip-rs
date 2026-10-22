use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{land_until_cast, tokens::{self, CITIZEN}};
use cantrip_oracle as oracle;

/// "When this creature enters, create a 1/1 green and white Citizen creature token."
fn citizen(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &CITIZEN, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&CITIZEN];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, citizen, "When this creature enters, create a 1/1 green and white Citizen creature token."),
        land_until_cast::ability::<0>(
            cost("{2}"),
            "{2}, Exile this card from your hand: Target land gains \"{T}: Add {R}, {G}, or {W}\" until this card is cast from exile. You may cast this card for as long as it remains exiled.",
        ),
    ],
    delayed: &[mana_ability(TAP, add_one_of(&[Color::Red, Color::Green, Color::White]), "{T}: Add {R}, {G}, or {W}.")],
    ..oracle::RAKISH_REVELERS
};
