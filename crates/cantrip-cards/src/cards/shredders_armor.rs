use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip_cost};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Sacrifice another nonland permanent" (CR 701.21a): any permanent you control that is not a land and not the Equipment itself.
fn another_nonland_permanent(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && !q.chars(id).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 701.3b: it is attached only if the target is still a legal one as this resolves; nothing is otherwise.
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip_cost(sacrifice(1, another_nonland_permanent), "Equip\u{2014}Sacrifice another nonland permanent. Activate only once each turn.")
            .once_each_turn(),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::SHREDDERS_ARMOR
};
