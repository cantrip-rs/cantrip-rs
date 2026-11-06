//! Choice captions are rules text too (CR 612). These native test definitions
//! exercise the shared choices printed by Primal Clay and Urborg; publication
//! card implementations are deliberately not part of this core fixture.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::choose;
use cantrip_script::{GameScript, run_on};
use cantrip_state::Obj;
use std::sync::OnceLock;

const YOU: PlayerId = PlayerId(0);

fn card_choice(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&WALKER);
    choose::one_of(
        g,
        item.controller,
        item.object,
        vec![Label::Card(card, 0), Label::Card(card, 1)],
    )?;
    Ok(())
}
fn option_choice(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&OPTIONS);
    choose::option(g, item.controller, item.object, card, 0..2)?;
    Ok(())
}
fn changed_during_resolution(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(
        item.object,
        item.object,
        EffectKind::ChangeText {
            from: Word::Subtype(Subtype::Swamp),
            to: Word::Subtype(Subtype::Island),
        },
        Duration::Indefinite,
    );
    card_choice(g, item)
}
fn return_clay(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let id = *g
        .state()
        .zone(Zone::graveyard(item.controller))
        .first()
        .expect("the forced graveyard card");
    g.put_onto_battlefield(id, item.controller, false)?;
    Ok(())
}
fn clay_entry(g: &mut dyn Rules, entry: &mut Entering) -> Result<(), Suspend> {
    choose::option_as_enters(g, entry, &CLAY, 0..3)
}
fn elf_copy(g: &mut dyn Rules, entry: &mut Entering) -> Result<(), Suspend> {
    let mut copy = Copiable::of(g.card_id(&CLAY));
    copy.words = Words::NONE.changed(Word::Subtype(Subtype::Wall), Word::Subtype(Subtype::Elf));
    entry.copy = Some(copy);
    Ok(())
}
static WALKER: CardDef = CardDef {
    name: "Choice Walker",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    labels: &["first strike", "swampwalk"],
    abilities: &[ability(
        Cost::NONE,
        card_choice,
        "Choose first strike or swampwalk.",
    )],
    ..CardDef::BLANK
};
static OPTIONS: CardDef = CardDef {
    name: "Choice Options",
    abilities: &[ability(
        Cost::NONE,
        option_choice,
        "Choose first strike or swampwalk.",
    )],
    ..WALKER
};
static MUTATING: CardDef = CardDef {
    name: "Choice Mutating",
    abilities: &[ability(
        Cost::NONE,
        changed_during_resolution,
        "Change this text, then choose.",
    )],
    ..WALKER
};
static RETURNER: CardDef = CardDef {
    name: "Choice Returner",
    abilities: &[ability(Cost::NONE, return_clay, "Return the forced card.")],
    ..WALKER
};
static CLAY: CardDef = CardDef {
    name: "Choice Clay",
    enters: Some(clay_entry),
    abilities: &[],
    labels: &[
        "3/3 artifact creature",
        "2/2 artifact creature with flying",
        "1/6 Wall artifact creature with defender",
    ],
    ..WALKER
};
static COPY: CardDef = CardDef {
    name: "Choice Copy",
    enters_as_copy: Some(elf_copy),
    abilities: &[],
    labels: &[],
    ..WALKER
};
static FILLER: CardDef = CardDef {
    name: "Choice Filler",
    oracle: true,
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([
            &WALKER, &OPTIONS, &MUTATING, &RETURNER, &CLAY, &COPY, &FILLER,
        ])
    })
}
fn game(def: &'static CardDef, zone: Zone) -> (Game, ObjectId) {
    let table = table();
    let mut game = Game::new(
        table,
        &Setup {
            decks: vec![vec![table.id_of(&FILLER); 30]; 2],
            ..Default::default()
        },
    );
    while !game.state.agenda.is_empty() {
        game.state.frame_remove(0);
    }
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    game.state.frame_push(Frame::new(FrameKind::Priority));
    (game, id)
}
fn change(game: &mut Game, id: ObjectId, from: Subtype, to: Subtype) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect {
        source: id,
        controller: YOU,
        timestamp,
        affected: id,
        kind: EffectKind::ChangeText {
            from: Word::Subtype(from),
            to: Word::Subtype(to),
        },
        duration: Duration::Indefinite,
        words: Words::NONE,
    });
}
fn start(game: &mut Game, id: ObjectId, action: ActionKind) {
    game.run();
    let request = game.pending().cloned().expect("priority");
    let RequestKind::Priority { options } = &request.kind else {
        panic!("{request:?}")
    };
    let option = options
        .iter()
        .find(|o| o.source == Some(id) && o.kind == action)
        .expect("the native action");
    game.answer(request.id, request.player, Answer::Act(Action::of(option)))
        .unwrap();
}
fn offered(game: &mut Game) -> Vec<Label> {
    for _ in 0..40 {
        game.run();
        let request = game.pending().cloned().expect("a native question");
        let answer = match &request.kind {
            RequestKind::ChooseModes {
                modes,
                eligible,
                count,
                ..
            } => {
                assert_eq!(*count, 1);
                assert_eq!(*eligible, (0..modes.len() as u8).collect::<Vec<_>>());
                return modes.clone();
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => Answer::Act(Action::of(
                options
                    .iter()
                    .find(|o| o.kind == ActionKind::ConfirmCasting)
                    .expect("free cast confirmed"),
            )),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("no choice");
}
fn texts(modes: &[Label]) -> Vec<String> {
    modes
        .iter()
        .map(|m| label_text_in(table(), m).to_string())
        .collect()
}

#[test]
fn card_labels_name_the_changed_landwalk() {
    let (mut g, id) = game(&WALKER, Zone::BATTLEFIELD);
    change(&mut g, id, Subtype::Swamp, Subtype::Island);
    start(&mut g, id, ActionKind::ActivateAbility);
    assert_eq!(texts(&offered(&mut g)), ["first strike", "islandwalk"]);
}
#[test]
fn option_labels_name_the_changed_landwalk() {
    let (mut g, id) = game(&OPTIONS, Zone::BATTLEFIELD);
    change(&mut g, id, Subtype::Swamp, Subtype::Island);
    start(&mut g, id, ActionKind::ActivateAbility);
    assert_eq!(texts(&offered(&mut g)), ["first strike", "islandwalk"]);
}
#[test]
fn an_entry_choice_uses_the_prospective_copys_words() {
    let (mut g, id) = game(&COPY, Zone::hand(YOU));
    start(&mut g, id, ActionKind::CastSpell);
    assert_eq!(
        texts(&offered(&mut g))[2],
        "1/6 Elf artifact creature with defender"
    );
}
#[test]
fn ordinary_card_labels_keep_their_identity() {
    let (mut g, id) = game(&WALKER, Zone::BATTLEFIELD);
    start(&mut g, id, ActionKind::ActivateAbility);
    let labels = offered(&mut g);
    assert_eq!(
        labels,
        [
            Label::Card(table().id_of(&WALKER), 0),
            Label::Card(table().id_of(&WALKER), 1)
        ]
    );
    assert_eq!(texts(&labels), ["first strike", "swampwalk"]);
}
#[test]
fn an_ordinary_entry_choice_keeps_its_indices_and_card_identity() {
    let (mut g, id) = game(&CLAY, Zone::hand(YOU));
    start(&mut g, id, ActionKind::CastSpell);
    let labels = offered(&mut g);
    assert_eq!(
        labels,
        (0..3)
            .map(|i| Label::Option(table().id_of(&CLAY), i))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        texts(&labels)[2],
        "1/6 Wall artifact creature with defender"
    );
}

#[test]
fn a_scenario_can_answer_by_the_captured_changed_caption() {
    let script = GameScript::parse(
        r#"
[Bot1 Deck]
Choice Copy
Choice Filler x20
[Bot2 Deck]
Choice Filler x20
[Actions]
Bot1.Hand.Choice Copy.Cast
Bot1.Pass
Bot2.Pass
Bot1.Modes(1/6 Elf artifact creature with defender)
[Expect]
Bot1.Battlefield.Count = 1
Bot1.Battlefield.Choice Clay.ChosenOption = 1/6 Elf artifact creature with defender
"#,
        "worded-entry-choice",
    )
    .unwrap();
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}\n{}", failure.message, failure.report);
    }
}

#[test]
fn the_captured_caption_does_not_follow_later_source_changes() {
    let (mut g, id) = game(&WALKER, Zone::BATTLEFIELD);
    change(&mut g, id, Subtype::Swamp, Subtype::Island);
    start(&mut g, id, ActionKind::ActivateAbility);
    let labels = offered(&mut g);
    change(&mut g, id, Subtype::Island, Subtype::Mountain);
    assert_eq!(
        Query::words(&g, id).subtype(Subtype::Swamp),
        Subtype::Mountain
    );
    assert_eq!(texts(&labels), ["first strike", "islandwalk"]);
}

#[test]
fn choosing_a_copy_keeps_the_printed_index_and_linked_card_through_undo_redo() {
    let (mut g, id) = game(&COPY, Zone::hand(YOU));
    start(&mut g, id, ActionKind::CastSpell);
    let labels = offered(&mut g);
    assert_eq!(texts(&labels)[2], "1/6 Elf artifact creature with defender");
    let before = g.at;
    let request = g.pending().cloned().unwrap();
    g.answer(request.id, request.player, Answer::Modes(vec![2]))
        .unwrap();
    let after = g.at;
    let permanent = *g.state.battlefield().first().expect("the copy entered");
    let expected = ChosenOption {
        card: table().id_of(&CLAY),
        index: 2,
    };
    assert_eq!(
        g.state.chosen_option(g.text_linked(permanent)),
        Some(expected)
    );
    assert_eq!(
        Query::words(&g, permanent).subtype(Subtype::Wall),
        Subtype::Elf
    );
    g.goto(before);
    let RequestKind::ChooseModes { modes, .. } =
        &g.pending().expect("entry question restored").kind
    else {
        panic!("choice")
    };
    assert_eq!(*modes, labels);
    g.goto(after);
    assert_eq!(
        g.state.chosen_option(g.text_linked(permanent)),
        Some(expected)
    );
}

fn entry(id: ObjectId, copy: Option<Copiable>) -> Entering {
    Entering {
        object: id,
        emblem: None,
        controller: YOU,
        tapped: false,
        counters: [0; CounterKind::COUNT],
        copy,
        prepared: false,
        riot_haste: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_number: None,
        chosen_object: None,
        attach: None,
        attach_player: None,
        x: 0,
        transformed: false,
        protector: None,
        read_ahead: None,
        instead: None,
        becomes: None,
        own: None,
        melded: None,
    }
}

#[test]
fn prospective_copy_replaces_old_copy_words_and_stack_effects_apply_once() {
    let (mut g, id) = game(&COPY, Zone::STACK);
    let mut old = Copiable::of(table().id_of(&CLAY));
    old.words = Words::NONE.changed(Word::Subtype(Subtype::Wall), Word::Subtype(Subtype::Goblin));
    g.state.update_obj(id, |o| o.copy = Some(old));
    let mut prospective = Copiable::of(table().id_of(&CLAY));
    prospective.words =
        Words::NONE.changed(Word::Subtype(Subtype::Wall), Word::Subtype(Subtype::Elf));
    change(&mut g, id, Subtype::Elf, Subtype::Shapeshifter);
    let expected = prospective.words.changed(
        Word::Subtype(Subtype::Elf),
        Word::Subtype(Subtype::Shapeshifter),
    );
    let words = Query::words_as_enters(&g, &entry(id, Some(prospective)));
    assert_eq!(
        words, expected,
        "the entire change list is captured exactly once"
    );
    assert_eq!(words.subtype(Subtype::Wall), Subtype::Shapeshifter);
    assert_eq!(Query::words(&g, id).subtype(Subtype::Wall), Subtype::Goblin);
}

#[test]
fn old_zone_effects_do_not_carry_but_copied_words_do() {
    let (mut g, id) = game(&COPY, Zone::graveyard(YOU));
    let mut prospective = Copiable::of(table().id_of(&CLAY));
    prospective.words =
        Words::NONE.changed(Word::Subtype(Subtype::Wall), Word::Subtype(Subtype::Elf));
    change(&mut g, id, Subtype::Elf, Subtype::Shapeshifter);
    assert_eq!(
        Query::words_as_enters(&g, &entry(id, Some(prospective))),
        prospective.words
    );
    assert_eq!(Query::words_as_enters(&g, &entry(id, None)), Words::NONE);
}

#[test]
fn an_explicitly_captured_label_is_not_changed_twice() {
    let words = Words::NONE.changed(Word::Subtype(Subtype::Wall), Word::Subtype(Subtype::Elf));
    let label = Label::Option(table().id_of(&CLAY), 2).with_words(words);
    assert_eq!(label.clone().with_words(words), label);
    assert_eq!(
        Label::Name(table().id_of(&CLAY)).with_words(words),
        Label::Name(table().id_of(&CLAY))
    );
    assert!(matches!(
        label_text_in(table(), &Label::Option(table().id_of(&CLAY), 0)),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn a_resolving_ability_uses_its_stack_text_after_changing_its_source() {
    let (mut g, id) = game(&MUTATING, Zone::BATTLEFIELD);
    start(&mut g, id, ActionKind::ActivateAbility);
    assert_eq!(texts(&offered(&mut g)), ["first strike", "swampwalk"]);
    assert_eq!(
        Query::words(&g, id).subtype(Subtype::Swamp),
        Subtype::Island
    );
}

#[test]
fn returning_from_an_old_zone_does_not_recapture_its_old_text_for_the_entry_choice() {
    let (mut g, source) = game(&RETURNER, Zone::BATTLEFIELD);
    let old = g
        .state
        .push_obj(Obj::new(table().id_of(&CLAY), YOU, Zone::graveyard(YOU)));
    g.state.zone_push(Zone::graveyard(YOU), old);
    change(&mut g, old, Subtype::Wall, Subtype::Elf);
    start(&mut g, source, ActionKind::ActivateAbility);
    assert_eq!(
        texts(&offered(&mut g))[2],
        "1/6 Wall artifact creature with defender"
    );
}
