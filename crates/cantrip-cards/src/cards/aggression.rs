use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_oracle as oracle;

/// Enchant a non-Wall creature (CR 702.5a).
fn enchanted_creature_has_first_strike_enchant(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall))) && q
        .chars(id).is(CardType::Creature)))
    )
}

/// "Enchanted creature has first strike and trample." (CR 604.1).
fn enchanted_creature_has_first_strike_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::FirstStrike);
    chars.keywords = chars.keywords.with(Keyword::Trample);
}

/// "At the beginning of the end step of enchanted creature's controller" (CR 603.2).
fn when_beginning_step_enchanted_creatures_controller(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::StepBegan { step } = event else { return None };
    let subject = Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .map(Target::Object)
        .unwrap_or(Target::None);
    if matches!(* step, Step::End) && subject != Target::None
        && Some(asker.source)
            .and_then(|o: ObjectId| q.state().obj(o).attached_to)
            .map(|o: ObjectId| q.controller(o)) == Some(q.state().turn.active)
    {
        return Fired::about(subject);
    }
    None
}

/// "At the beginning of the end step of enchanted creature's controller, destroy that creature if it didn't attack this turn." (CR 608.2, 701.8a).
fn destroy_creature_if_didnt_attack(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    if !g.history().attacked_this_turn(creature) {
        g.destroy(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::STEP_BEGAN,
                when_beginning_step_enchanted_creatures_controller,
            ),
            destroy_creature_if_didnt_attack,
            "At the beginning of the end step of enchanted creature's controller, destroy that creature if it didn't attack this turn.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_has_first_strike_apply_0,
            "Enchanted creature has first strike and trample.",
        ),
    ],
    enchant: Some(target(enchanted_creature_has_first_strike_enchant)),
    ..oracle::AGGRESSION
};
