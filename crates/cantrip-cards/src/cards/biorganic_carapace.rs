use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::modified::is_modified;
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER, THIS_ENTERS};
use cantrip_mechanics::{effects, statics, targets};
use cantrip_oracle as oracle;

/// "Draw a card for each modified creature you control" (CR 700.9, 121.1): counted as the ability resolves, the equipped creature
/// itself included while the Carapace is on it (an Equipment is a modification), and by the controller of the ability, the
/// creature's controller (CR 113.8, 603.3a).
fn draw_for_each_modified_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let modified = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature) && is_modified(g, id)).count();
    effects::draw(g, item, modified as u32)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 701.3b: it is attached only if the target is still a legal one as this resolves; nothing is otherwise.
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    // The equipped creature's own ability (CR 613.1f, 113.6), so its controller draws.
    delayed: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        draw_for_each_modified_creature,
        "Whenever this creature deals combat damage to a player, draw a card for each modified creature you control.",
    )],
    statics: &[
        static_ability(
            Layer::Ability,
            attached_object,
            statics::grant_delayed_ability,
            "Equipped creature has \"Whenever this creature deals combat damage to a player, draw a card for each modified creature you control.\" (Equipment, Auras you control, and counters are modifications.)",
        ),
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<2, 2>, "Equipped creature gets +2/+2.").continues(),
    ],
    ..oracle::BIORGANIC_CARAPACE
};
