use cantrip_api::*;
use cantrip_mechanics::statics::{have_menace, plus_pt, your_creatures};
use cantrip_mechanics::{class, effects, sneak};
use cantrip_oracle as oracle;

/// "Whenever a creature you control leaves the battlefield" (CR 603.10a), looked back on: the creature as it last was on the
/// battlefield, under the controller it last had, wherever it went.
fn creature_you_control_leaves(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. } if from.is(ZoneKind::Battlefield) && q.chars(*old).is(CardType::Creature) && q.obj(*old).controller == me.controller => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Level 2 and up (CR 716.2a): the Class's level as the layers ask for it.
fn your_creatures_at_level_two(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().obj(source).class_level >= 2 && your_creatures(q, source, affected, so_far)
}

/// Level 3 (CR 716.2a): "creature cards in your graveyard" that you cast, as the creature spell they are cast as (a creature
/// card with an Adventure is cast as a creature spell only through its creature face, CR 715.3).
fn sneak_castable(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> bool {
    class::level(q, asker.source) >= 3
        && caster == asker.controller
        && q.obj(card).owner == asker.controller
        && q.card(q.obj(card).card).is(CardType::Creature)
        && q.chars(card).is(CardType::Creature)
}

/// "Creature cards in your graveyard have sneak {3}{B}."
fn granted_sneak_cost(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> Option<Cost> {
    if !sneak_castable(q, asker, card, caster) {
        return None;
    }
    sneak::granted_cost(q, caster, mana("{3}{B}"))
}

/// "You may cast creature spells from your graveyard using their sneak abilities": the sneak a creature card has itself.
fn own_sneak_cost(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> Option<Cost> {
    if !sneak_castable(q, asker, card, caster) {
        return None;
    }
    sneak::own_cost(q, caster, card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_you_control_leaves),
            effects::each_opponent_loses_1_life,
            "Whenever a creature you control leaves the battlefield, each opponent loses 1 life.",
        ),
        class::level_two("{1}{B}", "{1}{B}: Level 2"),
        class::level_three("{B}", "{B}: Level 3"),
    ],
    statics: &[
        static_ability(Layer::PtModify, your_creatures_at_level_two, plus_pt::<1, 0>, "Creatures you control get +1/+0."),
        static_ability(Layer::Ability, your_creatures_at_level_two, have_menace, "Creatures you control have menace."),
    ],
    granted_ways: &[
        GrantedWay { text: "Sneak {3}{B}", ..sneak::granted(ZoneKind::Graveyard, granted_sneak_cost) },
        GrantedWay { text: "Sneak (own cost)", ..sneak::granted(ZoneKind::Graveyard, own_sneak_cost) },
    ],
    ..oracle::NINJA_TEEN
};
