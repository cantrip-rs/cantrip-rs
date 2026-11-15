use cantrip_api::*;
use cantrip_mechanics::{class, effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 0/4 white Wall creature token with defender that Builder's Talent makes: the colorless one of
/// `tokens::WALL_0_4_DEFENDER`, white.
static WALL: CardDef = CardDef { colors: Some(ColorSet::of(Color::White)), ..tokens::WALL_0_4_DEFENDER };

/// "When this Class enters, create a 0/4 white Wall creature token with defender."
fn create_a_wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WALL, 1)?;
    Ok(())
}

/// Level 2: "Whenever one or more noncreature, nonland permanents you control enter" (CR 603.2c, 603.6a), once the Class
/// is level 2 or more (CR 716.2a): the ability exists only then. An artifact, battle, enchantment or planeswalker (a token
/// too) under the Class's controller's control as it entered, as it is as it enters; a batch that entered together
/// triggers it once (`once_per_batch`).
fn noncreature_nonland_permanents_enter_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if class::level(q, me.source) >= 2
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && !q.chars(*new).is(CardType::Creature)
                && !q.chars(*new).is(CardType::Land) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, create_a_wall, "When this Class enters, create a 0/4 white Wall creature token with defender."),
        class::level_two("{W}", "{W}: Level 2"),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, noncreature_nonland_permanents_enter_at_level_two),
            effects::plus_one_counter_on_target,
            "Whenever one or more noncreature, nonland permanents you control enter, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .once_per_batch(),
        class::level_three("{4}{W}", "{4}{W}: Level 3"),
        triggered(
            class::BECOMES_LEVEL_THREE,
            effects::return_first_target_to_battlefield,
            "When this Class becomes level 3, return target noncreature, nonland permanent card from your graveyard to the battlefield.",
        )
        .targets(&[target(targets::noncreature_nonland_permanent_card_in_your_graveyard)]),
    ],
    ..oracle::BUILDERS_TALENT
};
pub static EXTRA: &[&CardDef] = &[&WALL];
