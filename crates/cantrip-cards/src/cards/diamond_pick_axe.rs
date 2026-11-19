use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The ability the equipped creature has (CR 613.1f), "Whenever this creature attacks, create a Treasure token", is the creature's
/// own: its controller creates the Treasure.
pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    delayed: &[triggered(THIS_ATTACKS, tokens::create_a_treasure, "Whenever this creature attacks, create a Treasure token.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, create a Treasure token.\"",
        ),
    ],
    ..oracle::DIAMOND_PICK_AXE
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
