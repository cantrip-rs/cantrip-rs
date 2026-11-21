use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{delayed, effects, targets, tokens};
use cantrip_oracle as oracle;

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
    spell: spell(&[targets::CREATURE_YOU_CONTROL], delayed::first_target_gains_its_delayed_ability_until_end_of_turn),
    delayed: &[triggered(
        THIS_DIES,
        return_it_tapped_then_create_a_wicked_role,
        "When this creature dies, return it to the battlefield tapped under its owner's control, then create a Wicked Role token attached to it.",
    )],
    ..oracle::NOT_DEAD_AFTER_ALL
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
