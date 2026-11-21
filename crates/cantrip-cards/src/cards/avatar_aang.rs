use cantrip_api::*;
use cantrip_mechanics::effects::{self, ask_may};
use cantrip_mechanics::triggers::EACH_UPKEEP;
use cantrip_oracle as oracle;

const TRANSFORM: Question = Question::new("TransformAang", "transform Aang, Master of Elements?", "You gain 4 life, draw four cards, put four +1/+1 counters on him, and he deals 4 damage to each opponent", "Transform", "Don't transform");

/// "Whenever you waterbend, earthbend, firebend, or airbend" (CR 701.65b, 701.66b, 701.67c, 702.189b, 603.2): its controller bent, in
/// any of the four ways.
fn you_bend(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if matches!(*event, Event::Bent { player, .. } if player == me.controller) { Fired::PLAIN } else { None }
}

/// "If you've done all four this turn" (CR 701.65-67, 702.189): the whole turn's history, not only what happened since Aang arrived
/// (the 2025-10-02 ruling), one `Event::Bent` of each kind by `player`.
fn done_all_four_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    let history = q.history();
    let done = |bending| history.select(EventQuery::kind(EventMask::BENT).this_turn().player(player)).any(|r| matches!(r.event, Event::Bent { bending: b, .. } if b == bending));
    [Bending::Water, Bending::Earth, Bending::Fire, Bending::Air].into_iter().all(done)
}

/// "Draw a card. Then if you've done all four this turn, transform Avatar Aang." The card is drawn first, so the bend that triggered this
/// is already in the history; the transformation does nothing if Aang is no longer that object (CR 400.7) or has transformed since the
/// ability went on the stack (CR 701.27f).
fn draw_then_maybe_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    if done_all_four_this_turn(g, item.controller) {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// "Spells you cast cost {W}{U}{B}{R}{G} less to cast. (This can reduce generic costs.)" (CR 601.2f, 118.7): each colored part comes off
/// the matching colored mana of the cost, and what is left of the reduction comes off the generic part.
fn spells_you_cast_cost_all_five_colors_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(_) = costed else { return CostDelta::NONE };
    if player == q.controller(source) { CostDelta::less_mana(mana("{W}{U}{B}{R}{G}")) } else { CostDelta::NONE }
}

/// "At the beginning of each upkeep, you may transform Aang, Master of Elements. If you do, you gain 4 life, draw four cards, put four
/// +1/+1 counters on him, and he deals 4 damage to each opponent." "If you do" is the transformation having happened: not if Aang has
/// left (CR 400.7) or has transformed since the ability went on the stack (CR 701.27f). The damage to the opponents is dealt together.
fn may_transform_and_wreck(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let aang = item.object;
    if !ask_may(g, item.controller, aang, Purpose::Ask(&TRANSFORM))? {
        return Ok(());
    }
    let before = g.obj(aang).transformed;
    g.transform_by(aang, item)?;
    if g.obj(aang).transformed == before {
        return Ok(());
    }
    g.gain_life(item.controller, 4);
    g.draw(item.controller, 4);
    g.add_counters(aang, CounterKind::PlusOne, 4);
    let hits: Vec<_> = g.state().opponents(item.controller).map(|opponent| (aang, Target::Player(opponent), 4)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(
        EACH_UPKEEP,
        may_transform_and_wreck,
        "At the beginning of each upkeep, you may transform Aang, Master of Elements. If you do, you gain 4 life, draw four cards, put four +1/+1 counters on him, and he deals 4 damage to each opponent.",
    )],
    cost_changes: &[spells_you_cast_cost_all_five_colors_less],
    ..oracle::AANG_MASTER_OF_ELEMENTS
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        cantrip_mechanics::firebending::trigger::<2>("Firebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)"),
        triggered(when_asked(EventMask::BENT, you_bend), draw_then_maybe_transform, "Whenever you waterbend, earthbend, firebend, or airbend, draw a card. Then if you've done all four this turn, transform Avatar Aang."),
    ],
    back: Some(&BACK),
    ..oracle::AVATAR_AANG
};
