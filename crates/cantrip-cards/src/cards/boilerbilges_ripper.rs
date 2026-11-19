use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature or enchantment. When you do" (CR 701.21a, 603.12): the choice is made as the triggered
/// ability resolves; a reflexive trigger is created only if the permanent was actually sacrificed.
fn sacrifice_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_sacrifice_when_you_do(g, item, select::another_creature_or_enchantment_permanents(select::up_to(1)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, sacrifice_for_damage, "When this creature enters, you may sacrifice another creature or enchantment. When you do, this creature deals 2 damage to any target.")],
    delayed: &[delayed_trigger(effects::damage_first_target::<2>, "When you do, this creature deals 2 damage to any target.").targets(&[targets::ANY])],
    ..oracle::BOILERBILGES_RIPPER
};
