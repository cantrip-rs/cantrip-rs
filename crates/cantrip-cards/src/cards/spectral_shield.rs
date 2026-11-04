use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +0/+2 and can't be the target of spells." (CR 604.1).
fn enchanted_creature_gets_0_2_rule_on(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    q.state().obj(asker.source).attached_to == Some(subject)
}

/// "Enchanted creature gets +0/+2 and can't be the target of spells." (CR 604.1).
fn enchanted_creature_gets_0_2_rule_by(
    _: &dyn LayerQuery,
    _: ObjectId,
    by: &Asker,
    _: &Chars,
) -> bool {
    by.spell
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            statics::plus_pt_saturating::<0, 2>,
            "Enchanted creature gets +0/+2 and can't be the target of spells.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    targeting_rules: &[
        TargetingRule {
            kind: TargetingRuleKind::Forbid(enchanted_creature_gets_0_2_rule_by),
            applies: enchanted_creature_gets_0_2_rule_on,
        },
    ],
    ..oracle::SPECTRAL_SHIELD
};
