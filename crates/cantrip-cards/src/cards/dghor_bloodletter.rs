use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Each player sacrifices a creature of their choice." Each player chooses in APNAP order and the chosen creatures are
/// sacrificed together (CR 101.4, 701.21a).
fn each_player_sacrifices_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::each_player_sacrifices(g, item.object, 1, |q, id| q.chars(id).is(CardType::Creature))?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(cantrip_mechanics::effects::other_creatures_get::<-1, -1>, "All other creatures get -1/-1 until end of turn."),
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
