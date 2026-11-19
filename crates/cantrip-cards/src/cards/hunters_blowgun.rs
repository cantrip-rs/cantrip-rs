use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, attached_object_during_your_turn, equip};
use cantrip_mechanics::statics::{have_deathtouch, have_reach, plus_pt};
use cantrip_oracle as oracle;

/// The "Otherwise" of "Equipped creature has deathtouch during your turn. Otherwise, it has reach." (CR 604.1, 613.1f): the
/// creature the Equipment is attached to, while the Equipment's controller is not the active player, asked again whenever
/// characteristics are computed.
fn attached_object_during_another_players_turn(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    view.state().turn.active != view.state().obj(source).controller && attached_object(view, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object_during_your_turn, have_deathtouch, "Equipped creature has deathtouch during your turn."),
        static_ability(Layer::Ability, attached_object_during_another_players_turn, have_reach, "Otherwise, it has reach."),
    ],
    ..oracle::HUNTERS_BLOWGUN
};
