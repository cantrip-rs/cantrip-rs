use cantrip_api::*;
use cantrip_mechanics::{amass::amass, damage_target, history::first_draw_in_own_draw_step, targets};
use cantrip_oracle as oracle;

/// "When this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw
/// steps": one ability with two conditions (CR 603.2), so the damage and the amass are always one resolution.
fn enters_or_opponent_draws(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Battlefield) => Fired::PLAIN,
        Event::Drew { player, .. } if *player != q.controller(source) && !first_draw_in_own_draw_step(q, *player) => Fired::PLAIN,
        _ => None,
    }
}

/// This creature deals 1 damage to any target. Then amass Orcs 1. A target that has become illegal takes both away (CR 608.2b).
fn shoot_and_amass(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    amass(g, item.controller, item.object, &ORC_ARMY, Subtype::Orc, 1)
}

/// The 0/0 black Orc Army creature token amass makes.
pub static ORC_ARMY: CardDef = CardDef {
    name: "Orc Army Token",
    token_name: Some(&[Subtype::Orc, Subtype::Army]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Orc, Subtype::Army]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&ORC_ARMY];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::DREW.0), enters_or_opponent_draws), shoot_and_amass, "When this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw steps, this creature deals 1 damage to any target. Then amass Orcs 1.")
        .targets(&[targets::ANY])],
    ..oracle::ORCISH_BOWMASTERS
};
