//! Mana abilities determine and lock the same total activation costs as other
//! activated abilities (CR 602.2b, 601.2f, 605.3a). Power Artifact and Forensic
//! Gadgeteer both reduce mana abilities of artifacts; Oppressive Rays can
//! increase mana ability costs. Availability, the mana planner and payment must agree.
use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
fn discount(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Ability(id, ..) if q.chars(id).is(CardType::Artifact) => CostDelta::less_to_one(2),
        _ => CostDelta::NONE,
    }
}
fn untapped_discount(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    match costed {
        Costed::Ability(id, ..) if q.state().obj(id).tapped => CostDelta::NONE,
        _ => discount(q, source, player, costed, targets),
    }
}
fn increase(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Ability(..)) { CostDelta::more(2) } else { CostDelta::NONE }
}
fn life_tax(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Ability(..)) { CostDelta::life(3) } else { CostDelta::NONE }
}
fn add_x(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, g.mana_ability_x().unwrap_or(0)); Ok(())
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }

static FILLER: CardDef = CardDef { name: "Mana change filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static PRISM: CardDef = CardDef {
    name: "Mana change prism", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{2}, {T}"), add_any_color(1), "{2}, {T}: Add one mana of any color.")],
    ..CardDef::BLANK
};
static COLORED: CardDef = CardDef {
    name: "Mana change colored", abilities: &[mana_ability(cost("{2}{U}, {T}"), add(Mana::G), "{2}{U}, {T}: Add {G}.")], ..PRISM
};
static ZERO: CardDef = CardDef {
    name: "Mana change zero", abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")], ..PRISM
};
static X: CardDef = CardDef {
    name: "Mana change X", abilities: &[mana_ability(cost("{X}, {T}"), add_by(add_x), "{X}, {T}: Add X {G}.").x_is_mana_amount()], ..PRISM
};
static DISCOUNT: CardDef = CardDef { name: "Mana change discount", types: TypeSet::of(&[CardType::Enchantment]), cost_changes: &[discount], ..CardDef::BLANK };
static UNTAPPED: CardDef = CardDef { name: "Mana change untapped", cost_changes: &[untapped_discount], ..DISCOUNT };
static INCREASE: CardDef = CardDef { name: "Mana change increase", cost_changes: &[increase], ..DISCOUNT };
static LIFE: CardDef = CardDef { name: "Mana change life", cost_changes: &[life_tax], ..DISCOUNT };
static SPELL: CardDef = CardDef {
    name: "Mana change blue spell", cost: mana("{U}"), has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK
};

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id); game.state.see_statics(); id
}
fn game(source: &'static CardDef, modifier: &'static CardDef, amount: u32) -> (Game, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&FILLER, &PRISM, &COLORED, &ZERO, &X, &DISCOUNT, &UNTAPPED, &INCREASE, &LIFE, &SPELL])));
    let deck = vec![cards.id_of(&FILLER); 20];
    let mut g = Game::new(cards, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.number = 1; t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let id = put(&mut g, source, Zone::BATTLEFIELD); put(&mut g, modifier, Zone::BATTLEFIELD);
    g.state.add_mana(YOU, Mana::C, amount, id, None, ManaLasts::Step, false);
    (g, id)
}
fn options(g: &mut Game) -> Vec<ActionOption> {
    g.run(); match &g.pending().expect("priority").kind {
        RequestKind::Priority { options } => options.clone(), other => panic!("{other:?}"),
    }
}
fn answer(g: &mut Game, answer: Answer) {
    let request = g.pending().cloned().unwrap(); g.answer(request.id, request.player, answer).unwrap(); g.run();
}
fn activate(g: &mut Game, source: ObjectId, color: Option<Color>) {
    let opts = options(g); let option = opts.iter().find(|o| o.source == Some(source)).expect("mana ability offered");
    let mut action = Action::of(option); action.color = color; answer(g, Answer::Act(action));
}
fn finish(g: &mut Game) {
    loop {
        let Some(r) = g.pending().cloned() else { break };
        match &r.kind {
            RequestKind::ChooseCostPayment { choices, .. } => {
                let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap)
                    .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana)).unwrap_or(0);
                answer(g, Answer::CostPayment(chosen));
            }
            RequestKind::PayCosts { announcing: false, options, .. } => {
                let option = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana)).expect("pool pays total");
                let action = Action::of(option); answer(g, Answer::Act(action));
            }
            _ => break,
        }
    }
}

#[test]
fn a_reduced_mana_ability_is_offered_with_only_its_reduced_cost() {
    let (mut g, source) = game(&PRISM, &DISCOUNT, 1);
    assert!(options(&mut g).iter().any(|o| o.source == Some(source)));
}
#[test]
fn payment_takes_the_reduced_amount_and_resolves_without_the_stack() {
    let (mut g, source) = game(&PRISM, &DISCOUNT, 2);
    activate(&mut g, source, Some(Color::Blue)); finish(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::C), 1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
    assert!(g.state.stack.is_empty()); assert!(g.state.obj(source).tapped);
}
#[test]
fn the_mana_planner_counts_a_conversion_at_the_reduced_cost() {
    let (mut g, _) = game(&PRISM, &DISCOUNT, 1); let spell = put(&mut g, &SPELL, Zone::hand(YOU));
    assert!(options(&mut g).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)));
}
#[test]
fn an_increase_changes_whether_the_ability_is_payable() {
    let (mut g, source) = game(&PRISM, &INCREASE, 3);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(source)));
}
#[test]
fn a_reduction_with_a_floor_never_makes_a_paid_ability_free() {
    let (mut g, source) = game(&PRISM, &DISCOUNT, 0);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(source)));
}
#[test]
fn that_floor_does_not_add_a_mana_cost_to_a_free_ability() {
    let (mut g, source) = game(&ZERO, &DISCOUNT, 0);
    activate(&mut g, source, None); assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
}
#[test]
fn the_colored_symbols_survive_a_generic_reduction() {
    let (mut g, source) = game(&COLORED, &DISCOUNT, 0);
    g.state.add_mana(YOU, Mana::U, 1, source, None, ManaLasts::Step, false);
    activate(&mut g, source, None); finish(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0); assert_eq!(g.state.player(YOU).pool.get(Mana::G), 1);
}
#[test]
fn the_total_is_locked_before_paying_the_tap_cost() {
    let (mut g, source) = game(&PRISM, &UNTAPPED, 2);
    activate(&mut g, source, Some(Color::Blue)); finish(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::C), 1);
}
#[test]
fn reductions_apply_after_the_activations_x_is_expanded() {
    let (mut g, source) = game(&X, &DISCOUNT, 1);
    activate(&mut g, source, None); answer(&mut g, Answer::Announce(3)); finish(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 3); assert_eq!(g.state.player(YOU).pool.get(Mana::C), 0);
}
#[test]
fn life_added_to_the_total_is_paid_as_well_as_mana() {
    let (mut g, source) = game(&PRISM, &LIFE, 2);
    activate(&mut g, source, Some(Color::Blue)); finish(&mut g);
    assert_eq!(g.state.player(YOU).life, 17);
}
#[test]
fn an_added_life_cost_cannot_be_offered_without_enough_life() {
    let (mut g, source) = game(&PRISM, &LIFE, 2); g.state.update_player(YOU, |p| p.life = 2);
    assert!(!options(&mut g).iter().any(|o| o.source == Some(source)));
}
