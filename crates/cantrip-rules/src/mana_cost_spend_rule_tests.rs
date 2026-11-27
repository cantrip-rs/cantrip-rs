//! A rule a resolved ability gives a player for paying one spell's mana cost (`EffectKind::ManaCostRuleForOneSpell`, CR 609.4b,
//! 611.2a), over test cards: whether a spell with {C} in its mana cost is offered for red mana. Mana of any type pays the
//! {C} (CR 106.1b: colorless is a type); mana as though it were of any color does not (no color stands for colorless). No
//! card in content has {C} in its mana cost, so North Star's scenarios, which hold the rest, cannot show it.

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

/// A spell with {C} in its mana cost: {U}{C}.
static COLORLESS_SYMBOL: CardDef = CardDef {
    name: "Mana Cost Rule Test Eldrazi",
    types: TypeSet::of(&[CardType::Artifact]),
    cost: ManaCost { symbols: [0, 1, 0, 0, 0, 1], ..ManaCost::ZERO },
    ..CardDef::BLANK
};

fn any_type(_: &dyn Query, _: &Asker, _: &Spending) -> AnyMana {
    AnyMana::Type
}
fn any_color(_: &dyn Query, _: &Asker, _: &Spending) -> AnyMana {
    AnyMana::Color
}

/// What gives the rules: its first is mana of any type, its second as though of any color.
static GIVER: CardDef = CardDef {
    name: "Mana Cost Rule Test Star",
    types: TypeSet::of(&[CardType::Artifact]),
    mana_cost_spend_rules: &[ManaCostSpendRule { text: "any type", lets: any_type }, ManaCostSpendRule { text: "any color", lets: any_color }],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// Whether the spell in your hand is offered in your main phase with two red mana in your pool, under the `rule`th of the
/// giver's rules (none for `None`).
fn offered(rule: Option<u8>) -> bool {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&COLORLESS_SYMBOL, &GIVER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let card = game.state.push_obj(Obj::new(table.id_of(&COLORLESS_SYMBOL), YOU, Zone::hand(YOU)));
    game.state.zone_push(Zone::hand(YOU), card);
    let giver = game.state.push_obj(Obj::new(table.id_of(&GIVER), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, giver);
    if let Some(index) = rule {
        let timestamp = game.state.timestamp();
        let kind = EffectKind::ManaCostRuleForOneSpell { card: table.id_of(&GIVER), index };
        game.state.effect_push(ContinuousEffect { source: giver, controller: YOU, timestamp, affected: giver, kind, duration: Duration::EndOfTurn, words: Words::NONE });
    }
    game.state.update_player(YOU, |p| p.pool = ManaPool([0, 0, 0, 2, 0, 0]));
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    priority_options(&mut cx(&mut game), YOU).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(card))
}

/// CR 609.4b, 106.1b: red pays the {U} and the {C} of the mana cost as mana of any type, only the {U} as though of any color,
/// and neither with no rule.
#[test]
fn a_rule_of_any_type_on_the_mana_cost_pays_its_colorless_symbol_and_one_of_any_color_does_not() {
    assert!(!offered(None), "{{U}}{{C}} is offered for {{R}}{{R}} with no rule");
    assert!(offered(Some(0)), "mana of any type does not pay the mana cost's {{C}}");
    assert!(!offered(Some(1)), "mana as though of any color pays the mana cost's {{C}}");
}
