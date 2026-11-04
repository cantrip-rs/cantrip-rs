use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::{effects, lands, statics, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{4}"), "Equip {4}")],
    // "Whenever this creature deals combat damage to a player, draw a card" (CR 603.2, 510.2, 113.6): the equipped creature's own
    // ability, so its controller draws, and only for damage dealt to a player.
    delayed: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        |g, item| effects::draw(g, item, 1),
        "Whenever this creature deals combat damage to a player, draw a card.",
    )],
    statics: &[
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Rogue) }>, "Equipped creature is a Rogue in addition to its other types."),
        static_ability(Layer::Ability, attached_object, statics::grant_delayed_ability, "Equipped creature has \"Whenever this creature deals combat damage to a player, draw a card.\"").continues(),
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 1>, "Equipped creature gets +1/+1.").continues(),
    ],
    ..oracle::THIEFS_KNIFE
};

pub static EXTRA: &[&CardDef] = &[&HERO];
