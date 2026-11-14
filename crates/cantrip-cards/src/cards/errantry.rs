use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +3/+0 and can only attack alone." (CR 604.1).
fn enchanted_creature_gets_3_0_rule(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.state().obj(asker.source).attached_to == Some(id)
}

/// "Enchanted creature gets +3/+0 and can only attack alone." (CR 604.1).
fn enchanted_creature_gets_3_0_rule_counted(
    _: &dyn Query,
    _: &Asker,
    _: ObjectId,
    _: ObjectId,
) -> bool {
    true
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            cantrip_mechanics::statics::plus_pt::<3, 0>,
            "Enchanted creature gets +3/+0 and can only attack alone.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    combat_groups: &[
        CombatGroupRule {
            role: CombatRole::Attacker,
            applies: enchanted_creature_gets_3_0_rule,
            counted: enchanted_creature_gets_3_0_rule_counted,
            others_only: false,
            at_least: 1,
            at_most: 1,
        },
    ],
    ..oracle::ERRANTRY
};
