use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Creatures you control gain haste until end of turn.
fn haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain(g, item, &[Keyword::Haste]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::may_discard_to_draw(g, item, 2), "When Bitter Reunion enters, you may discard a card. If you do, draw two cards."),
        ability(cost("{1}").and(SACRIFICE_THIS), haste, "{1}, Sacrifice Bitter Reunion: Creatures you control gain haste until end of turn."),
    ],
    ..oracle::BITTER_REUNION
};
