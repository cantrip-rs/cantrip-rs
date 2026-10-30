//! A replacement effect that has an arriving permanent enter the battlefield attacking (CR 614.1c, 508.4), over test cards,
//! as Kira Nerys, Firebrand's says: the permanent joins the combat as it arrives, tapped as the replacement says and attacking
//! what its controller chooses among what it could attack, and an instruction that names a defender for it afterwards has it
//! attack that instead - never putting it onto the battlefield a second time.

use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "If a creature you control would enter while you control an attacking creature, instead it enters tapped and attacking."
fn your_creature_enters_while_attacking(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.controller == asker.controller
            && q.chars(z.object).is(CardType::Creature))
        && q.state().battlefield().iter().any(|&id| q.controller(id) == asker.controller && q.is_attacking(id))
}

fn enters_tapped_and_attacking(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.tapped = true;
        z.attacking = true;
    }
}

static FIREBRAND: CardDef = CardDef {
    name: "Attacking Entry Test Firebrand",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(3),
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        your_creature_enters_while_attacking,
        enters_tapped_and_attacking,
        "it enters tapped and attacking while you control an attacking creature",
    )],
    ..CardDef::BLANK
};
static FOOTMAN: CardDef = CardDef {
    name: "Attacking Entry Test Footman",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static WALKER: CardDef = CardDef {
    name: "Attacking Entry Test Walker",
    types: TypeSet::of(&[CardType::Planeswalker]),
    loyalty: Some(3),
    ..CardDef::BLANK
};

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), controller, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// Player 0's turn in the declare attackers step, a Footman attacking player 1, the Firebrand beside it on the battlefield.
/// The card `entering` is put onto the battlefield from player 0's hand by the tests.
fn game(entering: &'static CardDef) -> (Game, &'static CardTable, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FIREBRAND, &FOOTMAN, &WALKER, entering])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::DeclareAttackers);
        t.attacked = true;
    });
    put(&mut game, table, &FIREBRAND, Zone::BATTLEFIELD, YOU);
    let already = put(&mut game, table, &FOOTMAN, Zone::BATTLEFIELD, YOU);
    game.state.set_attackers(vec![Attacker::new(already, Target::Player(THEM), THEM, TypeSet::NONE)]);
    let hand = put(&mut game, table, entering, Zone::hand(YOU), YOU);
    (game, table, hand)
}

#[test]
fn a_creature_a_replacement_has_arrive_attacking_joins_the_combat() {
    let (mut game, _, hand) = game(&FOOTMAN);
    let made = {
        let mut cx = cx(&mut game);
        ops::put_onto_battlefield_with(&mut cx, &[(hand, YOU, false)], ZoneReason::Other, &[]).expect("nothing is asked")[0]
    };
    assert!(game.state.obj(made).tapped, "the replacement's \"tapped\" is the arrival's too");
    let attacking: Vec<Attacker> = game.state.attackers.iter().copied().filter(|a| a.id == made).collect();
    assert_eq!(attacking.len(), 1, "it is put onto the battlefield once");
    // What it could attack: player 1 alone, nobody else and nothing they control that can be attacked.
    assert_eq!(attacking[0].target, Target::Player(THEM));
    assert_eq!(attacking[0].defending, THEM);
}

#[test]
fn an_instruction_that_names_a_defender_moves_an_arriving_attacker_to_it() {
    let (mut game, table, hand) = game(&FOOTMAN);
    let made = {
        let mut cx = cx(&mut game);
        ops::put_onto_battlefield_with(&mut cx, &[(hand, YOU, false)], ZoneReason::Other, &[]).expect("nothing is asked")[0]
    };
    let walker = put(&mut game, table, &WALKER, Zone::BATTLEFIELD, THEM);
    {
        let mut cx = cx(&mut game);
        ops::enter_attacking(&mut cx, &[made], Target::Object(walker));
    }
    let attacking: Vec<Attacker> = game.state.attackers.iter().copied().filter(|a| a.id == made).collect();
    assert_eq!(attacking.len(), 1, "an attacker already in the combat is not added again");
    assert_eq!(attacking[0].target, Target::Object(walker));
    assert_eq!(attacking[0].defending, THEM);
}
