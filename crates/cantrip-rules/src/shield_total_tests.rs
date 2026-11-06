//! CR 615.7: the prevention shields that apply to damage are taken together, however much they hold between them - two
//! shields of "the next 40,000 damage" hold 80,000, which no single shield's width counts.

use super::*;
use crate::Game as RulesGame;

const YOU: PlayerId = PlayerId(0);
const OPPONENT: PlayerId = PlayerId(1);

static SOURCE: CardDef = CardDef {
    name: "Shield total source probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn all_damage(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

static SHIELD: CardDef = CardDef {
    name: "Shield total probe",
    granted_preventions: &[GrantedPreventionDef { hint: "prevent the next damage", damage: "damage", of_it: None, applies: all_damage, then: None, text: "" }],
    ..CardDef::BLANK
};

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Two shields of 40,000 on the opponent: what they have left between them is 80,000, so of 40,000 and 30,000 damage from two
/// sources dealt to them at once, whose is prevented first is no choice - the shields hold more than both (CR 615.7) - and
/// none is asked.
#[test]
fn shields_hold_more_than_one_shields_width_between_them() {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &SHIELD])));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    // Turn 0 is no turn damage can be prevented in (`no_prevention_turn` starts at 0).
    game.state.update_turn(|t| t.number = 1);
    let mut sources = [ObjectId(0); 2];
    for source in &mut sources {
        *source = game.state.push_obj(Obj::new(cards.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, *source);
    }
    for timestamp in 1..=2 {
        game.state.effect_push(ContinuousEffect {
            source: sources[0],
            affected: sources[0],
            controller: OPPONENT,
            kind: EffectKind::Prevention { card: cards.id_of(&SHIELD), index: 0, to: Target::Player(OPPONENT), shield: Shield::Next(40_000) },
            duration: Duration::EndOfTurn,
            timestamp,
            words: Words::NONE,
        });
    }
    let mut cx = cx(&mut game);
    let hit = DamageEvent { source: sources[0], target: Target::Player(OPPONENT), amount: 40_000, combat: false, heal_marked: false, shielded: None };
    assert_eq!(replace::shields(&cx, &hit).map(|(_, left)| u32::from(left)), Some(80_000));
    let hits = [(sources[0], Target::Player(OPPONENT), 40_000), (sources[1], Target::Player(OPPONENT), 30_000)];
    assert_eq!(order_for_shields(&mut cx, &hits, false), vec![0, 1]);
    assert!(cx.g.pending().is_none(), "no player is asked whose damage is prevented first");
}
