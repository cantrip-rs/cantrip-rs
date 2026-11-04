use cantrip_api::*;
use cantrip_mechanics::each_player_sacrifices;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn creature_token(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.obj(id).token
}

fn nontoken_creature(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && !q.obj(id).token
}

fn enchantment(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Enchantment)
}

/// "Each player sacrifices a creature token of their choice." (CR 701.21a, 101.4): every player, you included, chooses in turn
/// order from the board as it is, and the chosen permanents are sacrificed together.
fn each_player_sacrifices_a_creature_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_sacrifices(g, item.object, 1, creature_token)?;
    Ok(())
}

/// "Each player sacrifices a nontoken creature of their choice."
fn each_player_sacrifices_a_nontoken_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_sacrifices(g, item.object, 1, nontoken_creature)?;
    Ok(())
}

/// "Each player sacrifices an enchantment of their choice."
fn each_player_sacrifices_an_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_sacrifices(g, item.object, 1, enchantment)?;
    Ok(())
}

/// "When Gaius van Baelsar enters, choose one —" (CR 700.2, 603.3c): the mode is chosen as the ability goes on the stack.
static MODAL: ModalDef = modes(&[
    mode(each_player_sacrifices_a_creature_token, "Each player sacrifices a creature token of their choice."),
    mode(each_player_sacrifices_a_nontoken_creature, "Each player sacrifices a nontoken creature of their choice."),
    mode(each_player_sacrifices_an_enchantment, "Each player sacrifices an enchantment of their choice."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When Gaius van Baelsar enters, choose one —")],
    ..oracle::GAIUS_VAN_BAELSAR
};
