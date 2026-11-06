//! What a divided total says it divides (CR 601.2d), the prompt's `Text` the client reads to draw the shares as damage or
//! as counters: an activated ability's in the words its stack item announces (a text-changing effect, CR 612), and a
//! triggered ability's, asked before it is on the stack (CR 603.3d), in the words it will have there.

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::prompt::prompt;
use cantrip_host::protocol::*;

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Divided Text Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature))
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "{0}: Distribute two +1/+1 counters among up to two target Elves." A text change turns the Elves into Goblins.
static ACTIVATED: CardDef = CardDef {
    name: "Divided Text Artifact",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::NONE, nothing, "{0}: Distribute two +1/+1 counters among up to two target Elves.")
        .targets(&[up_to(2, creature)])
        .divide(Divide { total: 2, reads: None })],
    ..CardDef::BLANK
};

/// "When this creature enters, it deals 3 damage divided as you choose among one or two target Elves."
static TRIGGERED: CardDef = CardDef {
    name: "Divided Text Elf",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_ENTERS, nothing, "When this creature enters, it deals 3 damage divided as you choose among one or two target Elves.")
        .targets(&[target(creature).between(1, 2)])
        .divide(Divide { total: 3, reads: None })],
    ..CardDef::BLANK
};

/// "Divided Text Bolt deals 3 damage divided as you choose among one or two target Elves." A spell's words belong to its
/// object, not to its stack item.
static SPELL: CardDef = CardDef {
    name: "Divided Text Bolt",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Instant]),
    text: "Divided Text Bolt deals 3 damage divided as you choose among one or two target Elves.",
    spell: spell(&[target(creature).between(1, 2)], nothing),
    divide: Some(Divide { total: 3, reads: None }),
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &ACTIVATED, &TRIGGERED, &SPELL])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn divide_prompt(g: &Game) -> ChooseDividedTargetsPrompt {
    let r = g.pending().cloned().expect("a question");
    match prompt(g, &r, &mut HandleSpace::new()).expect("a prompt").0 {
        PromptPayload::ChooseDividedTargetsPromptMessage(p) => p,
        other => panic!("expected the division, got {other:?}"),
    }
}

/// Takes the action offered for `source`, paying what is asked, until `done` says the question is the one wanted.
fn drive(g: &mut Game, source: ObjectId, done: impl Fn(&RequestKind) -> bool) {
    let mut acted = false;
    loop {
        g.run();
        let r = g.pending().cloned().expect("a question");
        if done(&r.kind) {
            return;
        }
        let answer = match &r.kind {
            RequestKind::Priority { options } if !acted && g.state.stack.is_empty() => {
                acted = true;
                Answer::Act(Action::of(options.iter().find(|o| o.source == Some(source)).expect("offered")))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => {
                Answer::Act(Action::of(options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays")))
            }
            other => panic!("unexpected {other:?}"),
        };
        g.answer(r.id, r.player, answer).expect("a legal answer");
    }
}

fn is_division(k: &RequestKind) -> bool {
    matches!(k, RequestKind::ChooseDividedTargets { .. })
}

/// KI-156: the trigger's own words, so a client draws counters as counters and damage as damage; its words as it will be put
/// on the stack, a text-changing effect's included.
#[test]
fn a_divided_trigger_says_its_text_before_it_is_on_the_stack() {
    let mut g = game();
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::C, 1, source, None, ManaLasts::Step, false);
    let elf = put(&mut g, &TRIGGERED, Zone::hand(YOU));
    drive(&mut g, elf, is_division);
    assert!(g.state.stack.last().is_none_or(|i| i.kind != StackKind::Triggered), "asked before the trigger is on the stack (CR 603.3d)");
    assert_eq!(divide_prompt(&g).text, "When this creature enters, it deals 3 damage divided as you choose among one or two target Elves.");

    // The words the ability will have there: the source's text changed, Elves to Goblins.
    let r = g.pending().cloned().expect("the division");
    let RequestKind::ChooseDividedTargets { trigger: Some((ability, _)), .. } = r.kind.clone() else { panic!("the trigger is carried: {:?}", r.kind) };
    let goblins = Words::NONE.changed(Word::Subtype(Subtype::Elf), Word::Subtype(Subtype::Goblin));
    let changed = Request { kind: match r.kind { RequestKind::ChooseDividedTargets { source, candidates, total, max_targets, optional, .. } => RequestKind::ChooseDividedTargets { source, candidates, total, max_targets, optional, trigger: Some((ability, goblins)) }, k => k }, ..r };
    let PromptPayload::ChooseDividedTargetsPromptMessage(p) = prompt(&g, &changed, &mut HandleSpace::new()).expect("a prompt").0 else { panic!("division") };
    assert_eq!(p.text, "When this creature enters, it deals 3 damage divided as you choose among one or two target Goblins.");
}

/// KI-157: an activated ability in the words its item announces.
#[test]
fn a_divided_activated_ability_says_the_words_its_item_announces() {
    let mut g = game();
    let artifact = put(&mut g, &ACTIVATED, Zone::BATTLEFIELD);
    put(&mut g, &TRIGGERED, Zone::BATTLEFIELD);
    drive(&mut g, artifact, is_division);
    assert_eq!(divide_prompt(&g).text, "{0}: Distribute two +1/+1 counters among up to two target Elves.");

    let at = g.state.stack.len() - 1;
    let mut item = g.state.stack[at];
    item.words = Words::NONE.changed(Word::Subtype(Subtype::Elf), Word::Subtype(Subtype::Goblin));
    g.state.stack_set(at, item);
    assert_eq!(divide_prompt(&g).text, "{0}: Distribute two +1/+1 counters among up to two target Goblins.");
}

/// A spell in the words its object has: a text-changing effect on it turns the Elves into Goblins.
#[test]
fn a_divided_spell_says_the_words_its_object_has() {
    let mut g = game();
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::C, 1, source, None, ManaLasts::Step, false);
    put(&mut g, &TRIGGERED, Zone::BATTLEFIELD);
    let bolt = put(&mut g, &SPELL, Zone::hand(YOU));
    drive(&mut g, bolt, is_division);
    assert_eq!(divide_prompt(&g).text, "Divided Text Bolt deals 3 damage divided as you choose among one or two target Elves.");

    // A text-changing effect on the spell on the stack (CR 612): a new object, the card having changed zones (CR 400.7). Its
    // stack item carries no words (`words: Words::NONE`); they belong to the object.
    let spell = g.state.stack.last().expect("the spell").object;
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect {
        source: spell,
        controller: YOU,
        timestamp,
        affected: spell,
        kind: EffectKind::ChangeText { from: Word::Subtype(Subtype::Elf), to: Word::Subtype(Subtype::Goblin) },
        duration: Duration::Indefinite,
        words: Words::NONE,
    });
    assert_eq!(divide_prompt(&g).text, "Divided Text Bolt deals 3 damage divided as you choose among one or two target Goblins.");
}
