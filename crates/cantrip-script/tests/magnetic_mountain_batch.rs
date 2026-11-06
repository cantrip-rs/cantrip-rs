//! The actual Magnetic Mountain caller untaps its chosen creatures in one
//! event batch (CR 603.2c, 701.26b), independently of final tapped-state checks.
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn creature_untapped(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::Untapped { permanent } if q.chars(permanent).is(CardType::Creature)
            && q.controller(permanent) == q.controller(source) => Fired::about(Target::Object(permanent)),
        _ => None,
    }
}
fn count_batch(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}
static ONE: CardDef = CardDef { name: "Mountain Batch One", oracle: true, has_cost: true,
    cost: cost("{U}").mana, colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1),
    abilities: &[triggered(when(EventMask::UNTAPPED, creature_untapped), count_batch,
        "Whenever one or more creatures you control become untapped, you gain 1 life.").once_per_batch()],
    ..CardDef::BLANK };
static TWO: CardDef = CardDef { name: "Mountain Batch Two", cost: cost("{W}").mana, ..ONE };

#[test]
fn the_actual_paid_upkeep_makes_one_batch_for_two_chosen_creatures() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&ONE, &TWO])));
    let retained = include_str!("../../../scenarios/magnetic-mountain-untaps-two-for-eight-mana.cantrip");
    let mut text = retained.replace("Merfolk of the Pearl Trident", ONE.name).replace("Zealous Guardian", TWO.name)
        .replace("[Expect]", "[T9 Bot1.Main1]\n[Expect]");
    // Both test-only creatures listen once per batch: two triggers, not four.
    text.push_str("\nBot1.Life = 22\n");
    let script = GameScript::parse(&text, "magnetic-mountain-native-batch").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}\n{}", failure.message, failure.report); }
}
