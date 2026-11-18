use cantrip_api::*;
use cantrip_mechanics::{discover, targets};
use cantrip_oracle as oracle;

/// "Counter target artifact or creature spell. Discover X, where X is that spell's mana value" (CR 701.6, 701.57a, 202.3e, 608.2h):
/// the value is read as the spell was on the stack, with the X chosen for it counted, before it is countered. It is discovered
/// whether or not the spell really was countered (it may be one that can't be); a spell whose only target has become illegal does
/// nothing at all (CR 608.2b), so there is no discover then.
fn counter_then_discover_its_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let x = g.mana_value(spell);
    g.counter(spell);
    g.end_sentence();
    discover::discover(g, item.controller, x)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE_SPELL], counter_then_discover_its_mana_value),
    ..oracle::HURL_INTO_HISTORY
};
