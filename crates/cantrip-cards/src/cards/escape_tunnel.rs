use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::library::{search_to_battlefield, Find};
use cantrip_oracle as oracle;

/// Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_battlefield(g, item.controller, item.object, Find::ONE, true, |q, c| {
        let chars = q.chars(c);
        chars.is(CardType::Land) && chars.supertypes.basic
    })?;
    Ok(())
}

/// "Target creature with power 2 or less" (CR 115.1).
fn power_two_or_less(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|p| p <= 2))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP.and(SACRIFICE_THIS), search, "{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle."),
        ability(TAP.and(SACRIFICE_THIS), effects::target_creature_cant_be_blocked_this_turn, "{T}, Sacrifice this land: Target creature with power 2 or less can't be blocked this turn.")
            .targets(&[target(power_two_or_less)]),
    ],
    ..oracle::ESCAPE_TUNNEL
};
