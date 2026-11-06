//! A cost change can name ninjutsu abilities as a class (CR 702.49a, `ActivatedDef::ninjutsu`): "ninjutsu abilities you activate
//! cost {1} less to activate" (Silver-Fur Master). No card the engine has asks yet, so a scenario file can't; these play one on
//! the real ninjas, with a test-only permanent for the change. It reaches all the cost path gives an activated ability of a
//! card in hand: whether the activation is offered (CR 602.2b), what is paid, increases before reductions and generic mana
//! only (CR 601.2f, 118.7a), and the priority row, which the client shows as it is (`CardActions`).

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::prompt::prompt;
use cantrip_host::protocol::*;
use cantrip_script::GameScript;

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Ninjutsu abilities you activate cost {1} less to activate."
fn one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(_, ability, _) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.activated_ability(ability).is_some_and(|d| d.ninjutsu) { CostDelta::less(1) } else { CostDelta::NONE }
}

static MASTER: CardDef = CardDef {
    name: "Ninjutsu Test Master",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    text: "Ninjutsu abilities you activate cost {1} less to activate.",
    cost_changes: &[one_less],
    ..CardDef::BLANK
};

/// "Abilities you activate from your hand cost {1} more to activate."
fn one_more(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(_, ability, _) = costed else { return CostDelta::NONE };
    if q.activated_ability(ability).is_some_and(|d| d.from == ZoneKind::Hand) { CostDelta::more(1) } else { CostDelta::NONE }
}

static TAXER: CardDef = CardDef {
    name: "Ninjutsu Test Taxer",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    text: "Abilities you activate from your hand cost {1} more to activate.",
    cost_changes: &[one_more],
    ..CardDef::BLANK
};

/// An ability of a card in hand that is no ninjutsu ability.
static CHANNELER: CardDef = CardDef {
    name: "Ninjutsu Test Channeler",
    oracle: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    text: "{2}: Nothing happens. Activate only from your hand.",
    abilities: &[ability(Cost::mana(mana("{2}")), nothing, "{2}: Nothing happens.").from(ZoneKind::Hand)],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new(cards().cards.iter().copied().chain([&MASTER, &TAXER, &CHANNELER]))))
}

/// Bot1 casts `permanents` (no more than two, of no cost), then Mox Jet, Mox Sapphire and Ornithopter, and has an Island: three
/// mana, with Ninja of the New Moon (Ninjutsu {3}{B}), Moon-Circuit Hacker (Ninjutsu {U}) and the Channeler in hand. The
/// Ornithopter attacks alone on turn 5, unblocked; then `lines` are played in the declare blockers step, and `expect` judged.
fn play(permanents: &[&str], lines: &str, expect: &str) -> Game {
    let deck: String = permanents.iter().map(|p| format!("{p}\n")).collect();
    let casts: String = permanents.iter().map(|p| format!("Bot1.Hand.{p}.Cast\nBot1.Pass\n")).collect();
    let script = format!(
        "[Bot1 Deck]
{deck}Island
Mox Jet
Mox Sapphire
Ornithopter
Ninja of the New Moon
Moon-Circuit Hacker
Ninjutsu Test Channeler
Island x13

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Island.Play
{casts}Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
[T5 Bot1.Main1]
Bot1.Attack(Ornithopter)
[T5 Bot1.Blockers]
{lines}

[Expect]
Turn = 5
{expect}
"
    );
    let script = GameScript::parse(&script, "ninjutsu cost changes").expect("parses");
    cantrip_script::run_on(&script, table()).map_err(|f| f.report).expect("plays").game
}

/// The descriptions of the priority rows of `name`'s abilities.
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

const MASTER_NAME: &str = "Ninjutsu Test Master";
const TAXER_NAME: &str = "Ninjutsu Test Taxer";
const NINJA: &str = "Ninja of the New Moon";
const HACKER: &str = "Moon-Circuit Hacker";
const CHANNELER_NAME: &str = "Ninjutsu Test Channeler";

/// The total a row says, the text in its last "(costs ...)", where it says one.
fn says(row: &str) -> Option<&str> {
    row.rfind("(costs ").map(|at| row[at + "(costs ".len()..].trim_end_matches(')'))
}

#[test]
fn the_row_says_the_reduced_total_of_a_ninjutsu_ability_and_of_nothing_else_in_hand() {
    let g = play(&[MASTER_NAME], "", "");
    let ninja = rows(&g, NINJA);
    assert_eq!(ninja.len(), 1, "{ninja:?}");
    assert_eq!(says(&ninja[0]), Some("{2}{B}"), "{{3}}{{B}} less {{1}}: {}", ninja[0]);
    // The Hacker's {U} has no generic mana to take off (CR 118.7a), so its total is its printed cost and the row says none.
    let hacker = rows(&g, HACKER);
    assert_eq!(hacker.len(), 1, "{hacker:?}");
    assert_eq!(says(&hacker[0]), None, "{}", hacker[0]);
    // An ability of a card in hand that is no ninjutsu ability costs what it prints.
    let channeler = rows(&g, CHANNELER_NAME);
    assert_eq!(channeler, ["{2}: Nothing happens."]);
}

#[test]
fn the_reduction_makes_the_activation_affordable_and_is_what_is_paid() {
    // With three mana and no reduction, the {3}{B} isn't offered (CR 602.2b).
    play(&[], &format!("! Bot1.Hand.{NINJA}.Activate(Ability: Ninjutsu; Mana: short)"), "");
    // With it, those three mana pay it exactly, and the Ninja is put onto the battlefield.
    play(
        &[MASTER_NAME],
        &format!(
            "Bot1.Hand.{NINJA}.Activate(Ability: Ninjutsu)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Island.Activate
Bot1.Choose(Return to hand 1 permanent)
Bot1.Choose(Ornithopter)
Bot1.Pass"
        ),
        &format!("Bot1.Battlefield.{NINJA}.Tapped = true\nBot1.Mana = "),
    );
}

#[test]
fn increases_apply_before_the_reduction() {
    // {U}, {1} more and then {1} less, is {U} (CR 601.2f); the other way round, the reduction would find no generic mana and
    // the total would be {1}{U}.
    let g = play(&[MASTER_NAME, TAXER_NAME], "", "");
    let hacker = rows(&g, HACKER);
    assert_eq!(says(&hacker[0]), None, "{}", hacker[0]);
    // And the tax alone is said, and the channel-style ability, which the reduction doesn't name, pays it.
    let g = play(&[TAXER_NAME], "", "");
    assert_eq!(says(&rows(&g, HACKER)[0]), Some("{1}{U}"));
    assert_eq!(says(&rows(&g, CHANNELER_NAME)[0]), Some("{3}"));
    // The one mana is all that is paid under both.
    play(
        &[MASTER_NAME, TAXER_NAME],
        &format!(
            "Bot1.Hand.{HACKER}.Activate(Ability: Ninjutsu)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Choose(Return to hand 1 permanent)
Bot1.Choose(Ornithopter)
Bot1.Pass"
        ),
        &format!("Bot1.Battlefield.{HACKER}.Tapped = true\nBot1.Battlefield.Mox Jet.Tapped = false\nBot1.Battlefield.Island.Tapped = false"),
    );
}
