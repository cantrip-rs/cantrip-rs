use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{statics, tokens};
use cantrip_oracle as oracle;

/// "When Okoye enters, create two 1/1 white Soldier creature tokens." (CR 111.2, 701.7a): together, under her controller.
fn two_soldiers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SOLDIER, 2)?;
    Ok(())
}

/// "Attacking creature tokens you control" (CR 506.4, 613.1f, layer 6): a creature token under the source's controller's control,
/// as the earlier layers made it, that is attacking now - declared as an attacker or put onto the battlefield attacking (CR 508.4)
/// - and so for as long as it is.
fn attacking_creature_tokens_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_creature_tokens(q, source, affected, so_far) && q.state().attackers.iter().any(|a| a.id == affected)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, two_soldiers, "When Okoye enters, create two 1/1 white Soldier creature tokens.")],
    statics: &[static_ability(
        Layer::Ability,
        attacking_creature_tokens_you_control,
        statics::have_first_strike,
        "Attacking creature tokens you control have first strike.",
    )],
    ..oracle::OKOYE_DORA_MILAJE_LEADER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
