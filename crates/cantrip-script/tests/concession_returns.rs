//! A player who concedes while their own announcement waits on a question leaves the game at once (CR 104.3a, 800.4a),
//! outside any run of the rules that could ask a question. What a permanent of theirs had exiled "until it leaves the
//! battlefield" comes back immediately after (CR 610.3): first thing as the game goes on, before the state triggers and
//! the state-based actions look at the board. On test-only cards.

use std::sync::OnceLock;

use cantrip_engine::*;

static WARDEN: CardDef = CardDef { name: "Probe Warden", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn plus_zero_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.toughness = c.toughness.map(|t| t + 1);
}

/// "All creatures get +0/+1."
static BULWARK: CardDef = CardDef {
    name: "Probe Bulwark",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, creature, plus_zero_one, "")],
    ..CardDef::BLANK
};

static SHELL: CardDef = CardDef { name: "Probe Shell", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

static CAPTIVE: CardDef = CardDef { name: "Probe Captive", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

fn on_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "{1}: Choose target creature." Its cost is asked for while the activation is being announced.
static POINTER: CardDef = CardDef {
    name: "Probe Pointer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { mana: ManaCost { generic: 1, ..ManaCost::ZERO }, ..Cost::NONE }, nothing, "{1}: Choose target creature.")
        .targets(&[target(on_creature)])],
    ..CardDef::BLANK
};

/// "{T}: Add {C}."
static SPRING: CardDef = CardDef {
    name: "Probe Spring",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&WARDEN, &BULWARK, &SHELL, &CAPTIVE, &POINTER, &SPRING, &FILLER]))
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table().id_of(def), owner, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn moved(game: &Game, id: ObjectId, from: ZoneKind, to: ZoneKind) -> usize {
    game.log
        .iter()
        .position(|r| matches!(r.event, Event::ZoneChange { old, from: f, to: t, .. } if old == id && f.is(from) && t.is(to)))
        .unwrap_or_else(|| panic!("{id:?} never moved from {from:?} to {to:?}"))
}

/// The first player's Warden holds the second player's Captive; the first player's Bulwark keeps the second player's
/// damaged 1/1 Shell alive. The first player concedes while paying for their own Pointer's ability: the
/// Warden and the Bulwark leave the game with them. The Captive is back before the next state-based check, which then puts
/// the Shell, a 1/1 with 1 damage once the Bulwark is gone, into its owner's graveyard.
#[test]
fn a_card_exiled_until_a_concessions_permanent_left_returns_before_the_next_check() {
    let filler = table().find("Filler").unwrap();
    let mut game = Game::new(table(), &Setup { decks: vec![vec![filler; 30]; 3], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let (you, them) = (PlayerId(0), PlayerId(1));
    let warden = put(&mut game, &WARDEN, you, Zone::BATTLEFIELD);
    put(&mut game, &BULWARK, you, Zone::BATTLEFIELD);
    let pointer = put(&mut game, &POINTER, you, Zone::BATTLEFIELD);
    put(&mut game, &SPRING, you, Zone::BATTLEFIELD);
    let shell = put(&mut game, &SHELL, them, Zone::BATTLEFIELD);
    game.state.update_obj(shell, |o| o.damage = 1);
    let captive = put(&mut game, &CAPTIVE, them, Zone::exile(them));
    let text = game.state.obj(warden).card;
    game.state.update_obj(captive, |o| o.exiled = Some(ExileLink { by: warden, text, acquired: Acquired::OWN, from: Zone::BATTLEFIELD, until_leaves: true, return_by: None, encoded: false }));
    let mut conceded = false;
    for _ in 0..100 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        match &request.kind {
            RequestKind::Priority { options } if request.player == you && !conceded => {
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(pointer)).expect("the Pointer");
                let mut action = Action::of(option);
                action.targets = vec![Target::Object(shell)];
                game.answer(request.id, request.player, Answer::Act(action)).unwrap();
            }
            RequestKind::PayCosts { .. } if request.player == you => {
                // The Pointer's activation waits on its cost: the first player concedes here.
                game.concede(you);
                conceded = true;
            }
            RequestKind::Priority { .. } if conceded => break,
            RequestKind::Priority { .. } => game.answer(request.id, request.player, Answer::Pass).unwrap(),
            other => panic!("unexpected question {other:?}"),
        }
    }
    assert!(conceded && game.state.player(you).lost);
    let returned = moved(&game, captive, ZoneKind::Exile, ZoneKind::Battlefield);
    let died = moved(&game, shell, ZoneKind::Battlefield, ZoneKind::Graveyard);
    assert!(returned < died, "the Captive came back at {returned}, the Shell died at {died}");
}
