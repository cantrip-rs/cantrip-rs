use cantrip_api::*;
use cantrip_mechanics::{each_opponent_sacrifices, pump, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+3 and gains trample until end of turn." Both changes are one instruction (CR 608.2c).
fn temba(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 3, 3);
        trample_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

fn nontoken_artifact_or_enchantment(q: &dyn Query, id: ObjectId) -> bool {
    !q.obj(id).token && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment))
}

/// "Each opponent sacrifices a nontoken artifact or enchantment of their choice" (CR 701.21a, 101.4).
fn shaka(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_sacrifices(g, item.controller, item.object, 1, nontoken_artifact_or_enchantment)
}

static MODAL: ModalDef = modes(&[
    mode(temba, "Target creature gets +3/+3 and gains trample until end of turn.").targets(&[targets::CREATURE]),
    mode(shaka, "Each opponent sacrifices a nontoken artifact or enchantment of their choice."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::DATHON_AND_PICARD_AT_EL_ADREL };
