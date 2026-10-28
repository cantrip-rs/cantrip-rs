use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "Equipped creature ... has reach" (CR 702.17, 613.1f) as a static ability's `StaticDef::apply`.
fn have_reach(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Reach);
}

/// "Whenever equipped creature becomes tapped" (CR 701.26a, 603.2): the creature this Equipment is attached to is tapped, by any
/// path that taps it. The creature is the trigger's subject, the source of the damage.
fn equipped_creature_becomes_tapped(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent } if q.attached_to(source) == Some(*permanent) => Fired::about(Target::Object(*permanent)),
        _ => None,
    }
}

/// "It deals 1 damage to each opponent" (CR 120.3, 102.2): the equipped creature, as it last existed if it has left (CR 113.7a),
/// is the source; each opponent of the Equipment's controller still in the game takes one event, all dealt together (CR 120.2).
fn ping_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    let hits: Vec<(ObjectId, Target, u16)> = g.state().opponents(item.controller).map(|p| (creature, Target::Player(p), 1)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::TAPPED, equipped_creature_becomes_tapped),
            ping_each_opponent,
            "Whenever equipped creature becomes tapped, it deals 1 damage to each opponent.",
        ),
        equip(mana("{1}"), "Equip {1}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_reach, "Equipped creature has reach."),
    ],
    ..oracle::HAWKEYES_BOW
};
