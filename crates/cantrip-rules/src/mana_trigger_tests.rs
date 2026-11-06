//! A triggered mana ability (CR 605.1b) triggers on the tapping of a permanent for mana however the permanent has it (CR
//! 613.1f): printed on the face that shows, given by a static ability, by a resolved effect, or by a copy effect's
//! exception. Over test cards: no printed card gives one.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

/// "Whenever this permanent is tapped for mana, add an additional {G}."
fn this_tapped_for_mana(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::TappedForMana { permanent, .. } if *permanent == source => Fired::PLAIN,
        _ => None,
    }
}

fn add_an_additional_g(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::G, 1);
    Ok(())
}

const EXTRA_G: AbilityDef =
    triggered(when(EventMask::TAPPED, this_tapped_for_mana), add_an_additional_g, "Whenever this permanent is tapped for mana, add an additional {G}.")
        .mana_ability(TriggeredMana::One(Mana::G));

static FILLER: CardDef = CardDef { name: "Mana Trigger Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "{T}: Add {G}."
static GROVE: CardDef = CardDef {
    name: "Mana Trigger Grove",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

fn is_grove(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.name == "Mana Trigger Grove"
}

fn grant(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let card = view.state().obj(source).card;
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "Groves have 'Whenever this permanent is tapped for mana, add an additional {G}.'"
static BLESSING: CardDef = CardDef {
    name: "Mana Trigger Blessing",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, is_grove, grant, "")],
    delayed: &[EXTRA_G],
    ..CardDef::BLANK
};

/// The ability, listed to be given by an effect or a copy's exception.
static GIFT: CardDef = CardDef { name: "Mana Trigger Gift", types: TypeSet::of(&[CardType::Sorcery]), delayed: &[EXTRA_G], ..CardDef::BLANK };

/// A double-faced card: its front taps for {G}; its back, "Mana Trigger Bloom", taps for {G} and prints the trigger.
static BLOOM: CardDef = CardDef {
    name: "Mana Trigger Bloom",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}."), EXTRA_G],
    ..CardDef::BLANK
};
static BUD: CardDef = CardDef { name: "Mana Trigger Bud", back: Some(&BLOOM), ..GROVE };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &GROVE, &BLESSING, &GIFT, &BUD])));
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

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Taps `source` for its mana from priority and gives back how much {G} you then have.
fn tap_for_green(g: &mut Game, source: ObjectId) -> u32 {
    g.run();
    let r = g.pending().cloned().expect("a question");
    let option = match &r.kind {
        RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source)).cloned(),
        _ => None,
    }
    .expect("its mana ability is offered");
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    g.run();
    g.state.player(YOU).pool.get(Mana::G)
}

/// A static ability gives the Grove the trigger: tapped for {G}, it adds another.
#[test]
fn a_triggered_mana_ability_a_static_ability_gives_triggers() {
    let mut g = game();
    put(&mut g, &BLESSING);
    let grove = put(&mut g, &GROVE);
    assert!(priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(grove)));
    assert_eq!(tap_for_green(&mut g, grove), 2);
}

/// A resolved effect gives the Grove the trigger (`EffectKind::AddAbility`).
#[test]
fn a_triggered_mana_ability_an_effect_gives_triggers() {
    let mut g = game();
    let grove = put(&mut g, &GROVE);
    let ability = AbilityRef::delayed(g.cards.id_of(&GIFT), 0);
    cantrip_api::Game::add_effect(&mut cx(&mut g), grove, grove, EffectKind::AddAbility(ability), Duration::EndOfTurn);
    assert_eq!(tap_for_green(&mut g, grove), 2);
}

/// The Grove is a copy of a Grove "except it has" the trigger (CR 707.9b).
#[test]
fn a_triggered_mana_ability_a_copys_exception_gives_triggers() {
    let mut g = game();
    let grove = put(&mut g, &GROVE);
    let values = Copiable::of(g.cards.id_of(&GROVE)).with_ability(AbilityRef::delayed(g.cards.id_of(&GIFT), 0));
    g.state.update_obj(grove, |o| o.copy = Some(values));
    assert_eq!(tap_for_green(&mut g, grove), 2);
}

/// A transformed double-faced permanent has its back face's trigger (CR 712.8e).
#[test]
fn a_triggered_mana_ability_a_back_face_prints_triggers() {
    let mut g = game();
    let bud = put(&mut g, &BUD);
    g.state.update_obj(bud, |o| o.transformed = true);
    assert_eq!(cx(&mut g).chars(bud).name, "Mana Trigger Bloom");
    assert_eq!(tap_for_green(&mut g, bud), 2);
}
