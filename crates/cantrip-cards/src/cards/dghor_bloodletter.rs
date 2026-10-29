use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "All other creatures get -1/-1 until end of turn." The set of affected creatures is determined as this effect begins
/// resolving (CR 611.2c); a creature that enters later is not among them.
fn all_other_creatures_get_minus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let affected: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for creature in affected {
            cantrip_mechanics::pump(g, item.object, creature, -1, -1);
        }
    });
    Ok(())
}

/// "Each player sacrifices a creature of their choice." Each player chooses in APNAP order and the chosen creatures are
/// sacrificed together (CR 101.4, 701.21a).
fn each_player_sacrifices_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::each_player_sacrifices(g, item.object, 1, |q, id| q.chars(id).is(CardType::Creature))?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(all_other_creatures_get_minus_one, "All other creatures get -1/-1 until end of turn."),
    mode(each_player_sacrifices_a_creature, "Each player sacrifices a creature of their choice."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one —\n• All other creatures get -1/-1 until end of turn.\n• Each player sacrifices a creature of their choice.",
    )],
    ..oracle::DGHOR_BLOODLETTER
};
