use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature you control gains 'When this creature dies, ...'" (CR 611.2a, 613.1f): the granted ability is the
/// card's delayed ability 0, a dies trigger of the creature's own until the cleanup step; nothing if the target is no longer legal as
/// the spell resolves (CR 608.2b).
fn gain_the_dies_trigger(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    instruction(g, |g| g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn));
    Ok(())
}

/// "Return it to the battlefield tapped under its owner's control, then create a Wicked Role token attached to it." (CR 110.2a, 400.7,
/// 603.10a, 303.4i): a Role is made for each permanent that actually came back (every card of a merged permanent, the ruling of
/// 2023-09-01), all created as one event (CR 101.4, 603.2c), under the ability's controller; none for a token (it ceased to exist), a card a replacement kept out, or one that came
/// back as something the Role can't enchant.
fn return_it_tapped_then_create_a_wicked_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned = effects::return_what_it_became(g, item.object, true, &[])?;
    g.end_sentence();
    tokens::create_roles(g, &tokens::WICKED_ROLE, item.controller, &returned)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], gain_the_dies_trigger),
    delayed: &[triggered(
        THIS_DIES,
        return_it_tapped_then_create_a_wicked_role,
        "When this creature dies, return it to the battlefield tapped under its owner's control, then create a Wicked Role token attached to it.",
    )],
    ..oracle::NOT_DEAD_AFTER_ALL
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
