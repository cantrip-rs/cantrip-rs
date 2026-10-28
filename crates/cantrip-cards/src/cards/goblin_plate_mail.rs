use cantrip_api::*;
use cantrip_mechanics::amass::amass_army;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_menace, plus_pt};
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Amass Goblins 1, then attach this Equipment to the amassed Army" (CR 701.47a, 701.47c, 701.3a): the Army the amass chose,
/// whether or not it received a counter, whether it was already there or the token it made. The Equipment moves to it only if it is
/// still on the battlefield and could equip it (CR 701.3b); the amass is done all the same.
fn amass_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let army = amass_army(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, 1)?;
    g.end_sentence();
    // No Army was chosen if the token could not be created: there is nothing to attach to.
    let Some(army) = army else { return Ok(()) };
    g.attach(item.object, army);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            amass_and_attach,
            "When this Equipment enters, amass Goblins 1, then attach this Equipment to the amassed Army.",
        ),
        equip(mana("{4}"), "Equip {4}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_menace, "Equipped creature has menace."),
    ],
    ..oracle::GOBLIN_PLATE_MAIL
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
