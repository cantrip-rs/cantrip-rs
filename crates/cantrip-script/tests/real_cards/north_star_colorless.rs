//! North Star's "you may spend mana as though it were mana of any type to pay that spell's mana cost" (CR 609.4b): mana
//! of any type pays a {C} symbol of the mana cost, which mana as though it were of any color would not (CR 106.1b: colorless
//! is a type and no color). No card in content has {C} in its mana cost, so no scenario can show it; a test spell with
//! {C} is cast with a Mox Ruby's red under North Star's own effect - two of them, of which the spell uses up one.

use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

/// A spell whose mana cost is {C}.
static COLORLESS_SYMBOL: CardDef = CardDef {
    name: "North Star Test Colorless Spell",
    types: TypeSet::of(&[CardType::Artifact]),
    cost: ManaCost { symbols: [0, 0, 0, 0, 0, 1], ..ManaCost::ZERO },
    ..CardDef::BLANK
};

/// Casts the {C} spell with a Mox Ruby, under `effects` of North Star's effects (two activations are two): whether it was
/// offered, whether it then resolved onto the battlefield with the Mox's red spent, and how many of the effects are left.
fn cast(effects: usize) -> (bool, bool, usize) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&COLORLESS_SYMBOL]))));
    let find = |name: &str| table.find(name).unwrap_or_else(|| panic!("no card {name}"));
    let mut g = Game::new(table, &Setup { decks: vec![vec![find("Island"); 20], vec![find("Island"); 20]], ..Default::default() });
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
    let spell = g.state.push_obj(Obj::new(find("North Star Test Colorless Spell"), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), spell);
    let mox = g.state.push_obj(Obj::new(find("Mox Ruby"), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, mox);
    let star = g.state.push_obj(Obj::new(find("North Star"), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, star);
    for _ in 0..effects {
        // What North Star's ability leaves as it resolves (`Game::mana_cost_rule_for_one_spell_this_turn`): its first rule, for one spell.
        let timestamp = g.state.timestamp();
        let kind = EffectKind::ManaCostRuleForOneSpell { card: find("North Star"), index: 0 };
        g.state.effect_push(ContinuousEffect { source: star, controller: YOU, timestamp, affected: star, kind, duration: Duration::EndOfTurn, words: Words::NONE });
    }
    let mut offered = false;
    for _ in 0..50 {
        g.run();
        let Some(request) = g.pending().cloned() else { break };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == YOU && !offered => {
                let Some(option) = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)) else { break };
                offered = true;
                Answer::Act(Action::of(option))
            }
            RequestKind::Priority { .. } if g.state.stack.is_empty() => break,
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                super::pay(options, remaining, *mana_abilities, options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(mox)))
            }
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    let resolved = g.state.zone(Zone::BATTLEFIELD).contains(&spell) || g.state.zone(Zone::BATTLEFIELD).iter().any(|&id| g.state.obj(id).card == find("North Star Test Colorless Spell"));
    let left = g.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::ManaCostRuleForOneSpell { .. })).count();
    (offered, resolved && g.state.obj(mox).tapped && g.state.player(YOU).pool == ManaPool::default(), left)
}

/// CR 609.4b, 106.1b: a Mox Ruby's red pays the {C} under North Star's effect, and is no {C} without it. Each activation is
/// "for one spell" (CR 611.2a): of two effects, the spell that needed one uses up that one alone.
#[test]
fn north_stars_mana_of_any_type_pays_a_colorless_symbol_of_the_mana_cost() {
    assert_eq!(cast(0), (false, false, 0), "{{C}} is offered for {{R}} with no effect");
    assert_eq!(cast(2), (true, true, 1), "North Star's mana of any type does not pay the mana cost's {{C}}, or one spell used up other than one effect");
}
