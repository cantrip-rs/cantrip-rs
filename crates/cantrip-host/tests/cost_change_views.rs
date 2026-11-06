//! An ability's total cost at the table (CR 601.2f by way of 602.2b and 605.3): where the cost changes in play make it other
//! than the printed cost, the priority row says what it costs now - a mana ability's as much as one that uses the stack, and
//! an ability with no target as much as one whose cost depends on its target - and a row nothing changes says nothing.

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::prompt::prompt;
use cantrip_host::protocol::*;

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Cost View Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Activated abilities of artifacts cost {3} more to activate."
fn three_more(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Ability(id, ..) if q.chars(id).is(CardType::Artifact) => CostDelta::more(3),
        _ => CostDelta::NONE,
    }
}

static TAXER: CardDef = CardDef {
    name: "Cost View Taxer",
    cost: mana("{2}"),
    types: TypeSet::of(&[CardType::Enchantment]),
    text: "Activated abilities of artifacts cost {3} more to activate.",
    cost_changes: &[three_more],
    ..CardDef::BLANK
};

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "{T}: Add {W}." and "{1}: Nothing happens."
static STONE: CardDef = CardDef {
    name: "Cost View Stone",
    cost: mana("{1}"),
    types: TypeSet::of(&[CardType::Artifact]),
    text: "{T}: Add {W}.\n{1}: Nothing happens.",
    abilities: &[mana_ability(TAP, add(Mana::W), "{T}: Add {W}."), ability(Cost::mana(mana("{1}")), nothing, "{1}: Nothing happens.")],
    ..CardDef::BLANK
};

/// "{T}: Add {W}." on a land, which the taxer leaves alone.
static FIELD: CardDef = CardDef {
    name: "Cost View Field",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    text: "{T}: Add {W}.",
    abilities: &[mana_ability(TAP, add(Mana::W), "{T}: Add {W}.")],
    ..CardDef::BLANK
};

fn game(taxed: bool) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &TAXER, &STONE, &FIELD])));
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
    let mut put = |def: &'static CardDef| {
        let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
    };
    put(&STONE);
    put(&FIELD);
    if taxed {
        put(&TAXER);
    }
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::C, 4, source, None, ManaLasts::Step, false);
    g.run();
    g
}

/// The descriptions of the priority rows of `name`'s abilities, by their text.
fn rows(g: &Game, name: &str) -> Vec<String> {
    let r = g.pending().cloned().expect("a question");
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let PromptPayload::PriorityPromptMessage(p) = prompt(g, &r, &mut HandleSpace::new()).expect("a prompt").0 else { panic!("priority") };
    options
        .iter()
        .zip(&p.options)
        .filter(|(o, _)| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| g.chars(s).name == name))
        .map(|(_, m)| m.description.clone())
        .collect()
}

#[test]
fn a_taxed_ability_row_says_its_total_mana_abilities_included() {
    let taxed = game(true);
    let stone = rows(&taxed, "Cost View Stone");
    assert_eq!(stone, ["{T}: Add {W}. (costs {3})", "{1}: Nothing happens. (costs {4})"], "the tap for mana costs {{3}} now, the {{1}} {{4}}");
    // The land is no artifact: its row says nothing of a cost.
    assert!(rows(&taxed, "Cost View Field").iter().all(|d| !d.contains("costs")));
    // With no tax, nothing is said.
    assert!(rows(&game(false), "Cost View Stone").iter().all(|d| !d.contains("costs")));
}
