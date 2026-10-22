use cantrip_api::*;
use cantrip_mechanics::{damage_target, each_opponent_keeps_one_of_each_type, exile_and_return_transformed, targets, triggers, effects, tokens};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileAndReturn", "exile Ajani, then return him to the battlefield transformed?", "He returns as Ajani, Nacatl Avenger", "Exile and transform", "Don't");

/// The 2/1 white Cat Warrior token both faces make.
pub static CAT_WARRIOR: CardDef = CardDef {
    name: "Cat Warrior Token",
    token_name: Some(&[Subtype::Cat, Subtype::Warrior]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Cat, Subtype::Warrior]),
    power: Some(2),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn create_cat(g: &mut dyn Game, controller: PlayerId) -> Result<bool, Suspend> {
    Ok(!tokens::create(g, controller, &CAT_WARRIOR, 1)?.is_empty())
}

fn enters_make_cat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create_cat(g, item.controller)?;
    Ok(())
}

/// "Whenever one or more other Cats you control die" (CR 603.2c): the Cats are judged as they were on the
/// battlefield, and the ability, being `once_per_batch`, triggers once for those that died together.
fn cats_die(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.obj(*old).controller == q.obj(source).controller
                && q.chars(*old).has_subtype(q.words(source).subtype(Subtype::Cat)) =>
        {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// "You may exile Ajani, then return him to the battlefield transformed under his owner's control." The choice is
/// asked whether or not he is still there; if he is not, there is nothing to exile (CR 400.7).
fn exile_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

// --- Ajani, Nacatl Avenger ---------------------------------------------------------------------------------------

fn plus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for id in g.permanents_of(item.controller) {
        if g.chars(id).has_subtype(g.words(item.object).subtype(Subtype::Cat)) {
            g.add_counters(id, CounterKind::PlusOne, 1);
        }
    }
    Ok(())
}

/// "If you control a red permanent other than Ajani."
fn controls_other_red(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| id != asker.source && q.chars(id).colors.has(q.words(asker.source).color(Color::Red)))
}

/// "0: Create a 2/1 white Cat Warrior creature token. When you do, if you control a red permanent other than Ajani,
/// he deals damage equal to the number of creatures you control to any target." The reflexive trigger exists only if
/// the token was created (CR 603.12), and, like any triggered ability, only triggers if its condition holds (CR 603.4), checked again as it resolves.
fn zero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = create_cat(g, item.controller)?;
    if created {
        let me = g.card_id(&BACK);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(me, 0), Target::None);
    }
    Ok(())
}

fn damage_for_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = cantrip_mechanics::creatures_you_control(g, item.controller);
    damage_target(g, item, 0, creatures as u16);
    Ok(())
}

/// "-4: Each opponent chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland
/// permanents they control, then sacrifices the rest." Each opponent in turn order keeps one of each kind they have
/// more than one of (asked kind by kind), and what is not kept is sacrificed together.
fn minus_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_keeps_one_of_each_type(
        g,
        item.controller,
        item.object,
        &[CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker],
        |chars| !chars.is(CardType::Land),
    )
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), plus_two, "+2: Put a +1/+1 counter on each Cat you control."),
        ability(loyalty(0), zero, "0: Create a 2/1 white Cat Warrior creature token. When you do, if you control a red permanent other than Ajani, he deals damage equal to the number of creatures you control to any target."),
        ability(loyalty(-4), minus_four, "\u{2212}4: Each opponent chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest."),
    ],
    delayed: &[delayed_trigger(damage_for_creatures, "When you do, if you control a red permanent other than Ajani, he deals damage equal to the number of creatures you control to any target.")
        .targets(&[targets::ANY])
        .condition(controls_other_red)],
    ..oracle::AJANI_NACATL_AVENGER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, enters_make_cat, "When Ajani enters, create a 2/1 white Cat Warrior creature token."),
        triggered(when(EventMask::ZONE_CHANGE, cats_die), exile_and_return, "Whenever one or more other Cats you control die, you may exile Ajani, then return him to the battlefield transformed under his owner's control.")
            .once_per_batch(),
    ],
    back: Some(&BACK),
    ..oracle::AJANI_NACATL_PARIAH
};

pub static EXTRA: &[&CardDef] = &[&CAT_WARRIOR];
