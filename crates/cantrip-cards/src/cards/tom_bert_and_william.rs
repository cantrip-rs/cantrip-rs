use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{discard_chosen, enduring};
use cantrip_oracle as oracle;

/// "Draw cards equal to the sacrificed creature's power, then discard a card" (CR 121.1, 701.9, 608.2h): the power the creature had
/// as it last existed on the battlefield, pumps and counters included (the ruling); none for a negative power. The creature is the
/// one this activation's cost sacrificed (`Query::cost_objects`), whatever other activations are on the stack. The discard is made
/// all the same, even after drawing nothing.
fn draw_its_power_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.cost_objects(item).first().and_then(|&creature| g.chars(creature).power).unwrap_or(0).max(0) as u32;
    g.draw(item.controller, power);
    g.end_sentence();
    discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "They're an artifact": what it returns as (`EntersAs`), from the moment it enters (CR 614.12): exactly the card type artifact, so
/// it is no creature (CR 205.1a) and loses its creature types, and stays one for as long as it is that object (CR 611.2a, 613.1d).
static AN_ARTIFACT: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Artifact])), ..EntersAs::NONE };

/// "Return them to the battlefield. They're an artifact": under the control of the player who controls this ability - the one who
/// controlled the creature as it died (CR 603.3a, 603.10a) - since the text names no other (CR 110.2a); a card that has since left
/// the graveyard is another object and returns nothing (CR 400.7).
fn return_as_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = g.found_after_move(item.object) else { return Ok(()) };
    g.put_onto_battlefield_as(card, item.controller, item, &AN_ARTIFACT)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(sacrifice(1, filters::another_creature)),
            draw_its_power_then_discard,
            "{1}, Sacrifice another creature: Draw cards equal to the sacrificed creature's power, then discard a card.",
        ),
        // "If they were a creature" (CR 603.4, 700.4): an intervening "if" about what it last was on the battlefield, so it does
        // not trigger when it dies as the artifact it has become.
        triggered(
            when(EventMask::ZONE_CHANGE, enduring::dies_as_a_creature),
            return_as_an_artifact,
            "When Tom, Bert, and William die, if they were a creature, return them to the battlefield. They're an artifact.",
        ),
    ],
    ..oracle::TOM_BERT_AND_WILLIAM
};
