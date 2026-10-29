use cantrip_api::*;
use cantrip_mechanics::tokens::MUTAGEN;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

fn create_mutagen(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MUTAGEN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_mutagen, "When Ray Fillet enters, create a Mutagen token."),
        ability(
            // Only +1/+1 counters pay it, from any creature you control (CR 118.3, 122.1).
            cost("{2}").and(remove_kind_counters_among(CounterKind::PlusOne, 1, targets::filters::creature)),
            |g, item| effects::draw(g, item, 1),
            "{2}, Remove a +1/+1 counter from a creature you control: Draw a card.",
        ),
    ],
    ..oracle::RAY_FILLET_MAN_RAY
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
