use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Create a tapped token that's a copy of target token you control." (CR 707.2, 707.9a, 111.1): the target's copiable
/// values as the ability resolves - a token that has since left the battlefield is not there to copy (CR 608.2b) - and the
/// new one enters tapped (CR 111.1, 614.1c: it is put onto the battlefield tapped, never untapped first).
fn tapped_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(target);
    g.create_token_copies_tapped(values, item.controller, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this artifact enters, create a Food token."),
        ability(cost("{2}, {T}"), tapped_copy, "{2}, {T}: Create a tapped token that's a copy of target token you control.")
            .targets(&[targets::TOKEN_YOU_CONTROL]),
    ],
    ..oracle::REPLICATOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
