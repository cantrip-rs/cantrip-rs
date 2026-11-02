use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_oracle as oracle;

/// "Enchanted creature has hexproof and can't be blocked by more than one creature." (CR 608.2).
fn enchanted_creature_has_hexproof_cant_enchant(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Enchanted creature has hexproof and can't be blocked by more than one creature." (CR 604.1).
fn enchanted_creature_has_hexproof_cant_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Hexproof);
}

/// "Enchanted creature has hexproof and can't be blocked by more than one creature." (CR 604.1, 701.3a).
fn enchanted_creature_has_hexproof_cant_rule(
    q: &dyn Query,
    source: ObjectId,
    id: ObjectId,
) -> Option<usize> {
    let asker = &Asker::new(source, q.controller(source));
    (q.state().obj(asker.source).attached_to == Some(id)).then_some(1)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_has_hexproof_cant_apply_0,
            "Enchanted creature has hexproof and can't be blocked by more than one creature.",
        ),
    ],
    enchant: Some(target(enchanted_creature_has_hexproof_cant_enchant)),
    blocker_limits: &[enchanted_creature_has_hexproof_cant_rule],
    ..oracle::ALPHA_AUTHORITY
};
