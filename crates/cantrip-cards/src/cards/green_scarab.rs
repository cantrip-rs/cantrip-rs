use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature can't be blocked by green creatures. Enchanted creature gets +2/+2 as long as an opponent controls a green permanent." (CR 604.1).
fn enchanted_creature_cant_be_blocked_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(q.state().obj(asker.source).attached_to == Some(attacker)
        && (q.chars(blocker).colors.has(q.words(asker.source).color(Color::Green))
            && q.chars(blocker).is(CardType::Creature)))
}

/// "Enchanted creature gets +2/+2 as long as an opponent controls a green permanent." (CR 604.1).
fn enchanted_creature_gets_2_2_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    chars: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .into_iter()
        .any(|bound_0: PlayerId| {
            q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.controller(id) == Some(bound_0).unwrap_or(PlayerId(u8::MAX))
                        && q
                            .chars(id)
                            .colors
                            .has(q.words(asker.source).color(Color::Green))
                })
                .count() >= 1
        })) && attach::attached_object(q, source, affected, chars)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            enchanted_creature_gets_2_2_affects,
            statics::plus_pt_saturating::<2, 2>,
            "Enchanted creature gets +2/+2 as long as an opponent controls a green permanent.",
        ),
    ],
    block_rules: &[enchanted_creature_cant_be_blocked_rule],
    enchant: Some(targets::CREATURE),
    ..oracle::GREEN_SCARAB
};
