use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_vigilance, plus_pt};
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Tap target creature defending player controls": a target no longer legal is not tapped (CR 608.2b).
fn tap_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(defender) = targets::object(g, item, 0) {
        g.tap(defender);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::EQUIPPED_CREATURE_ATTACKS_SUBJECT, tap_target, "Whenever equipped creature attacks, tap target creature defending player controls.")
            .targets(&[targets::CREATURE_DEFENDING_PLAYER_CONTROLS]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<0, 8>, "Equipped creature gets +0/+8."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
    ],
    ..oracle::CAPTAIN_AMERICAS_SHIELD
};
