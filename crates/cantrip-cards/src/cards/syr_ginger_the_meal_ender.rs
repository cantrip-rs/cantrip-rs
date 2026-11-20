use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "As long as an opponent controls a planeswalker" (CR 604.1, 613.1f) as `StaticDef::affects`: only the source itself, and only
/// while a permanent an opponent of its controller controls is a planeswalker, as the earlier layers made it.
fn itself_while_an_opponent_controls_a_planeswalker(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    let opponents: Vec<PlayerId> = state.opponents(state.obj(source).controller).collect();
    affected == source && state.battlefield().iter().any(|&id| opponents.contains(&state.obj(id).controller) && q.chars(id).is(CardType::Planeswalker))
}

/// "... has trample, hexproof, and haste" (CR 702.19, 702.11, 702.10, layer 6).
fn have_trample_hexproof_and_haste(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Trample).with(Keyword::Hexproof).with(Keyword::Haste);
}

/// "Whenever another artifact you control is put into a graveyard from the battlefield" (CR 603.10a): an artifact other than this
/// one, as it last was and under the controller it last had.
fn another_artifact_you_control_to_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if *old == me.source => None,
        _ => triggers::type_you_control_to_graveyard::<{ CardType::Artifact as u8 }>(q, me, event),
    }
}

/// "Put a +1/+1 counter on Syr Ginger and scry 1" (CR 122.1, 701.22a, 608.2c): two instructions, followed in order, the scry made
/// whatever became of the counter's target.
fn counter_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    effects::scry(g, item, 1)
}

/// "You gain life equal to its power" (CR 119.3, 113.7a): the power Syr Ginger had on the battlefield, as it was sacrificed
/// as a cost; nothing for a power of zero or less, or for none (a noncreature permanent has no power, CR 208.3).
fn gain_life_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0).max(0);
    effects::gain_life(g, item, power)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself_while_an_opponent_controls_a_planeswalker,
        have_trample_hexproof_and_haste,
        "Syr Ginger has trample, hexproof, and haste as long as an opponent controls a planeswalker.",
    )],
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_artifact_you_control_to_graveyard),
            counter_and_scry,
            "Whenever another artifact you control is put into a graveyard from the battlefield, put a +1/+1 counter on Syr Ginger and scry 1.",
        ),
        ability(cost("{2}").and(TAP).and(SACRIFICE_THIS), gain_life_equal_to_its_power, "{2}, {T}, Sacrifice Syr Ginger: You gain life equal to its power."),
    ],
    ..oracle::SYR_GINGER_THE_MEAL_ENDER
};
