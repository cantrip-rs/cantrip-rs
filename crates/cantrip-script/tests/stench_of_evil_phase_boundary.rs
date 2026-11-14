//! A printed Oubliette/Jihad interaction with a test-only static that makes Oubliette a Plains on the battlefield.
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool { source == affected }
fn plains(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.types = chars.types.with(CardType::Land);
    chars.subtypes = chars.subtypes.with(Subtype::Plains);
}
static PLAINS_ABILITY: &[StaticDef] = &[static_ability(Layer::Type, itself, plains, "This is a Plains land in addition to its other types.")];

#[test]
fn phased_jihad_triggers_before_stench_payment() {
    let mut fixture = *cards().get(cards().find("Oubliette").unwrap());
    fixture.name = "Stench Test Oubliette";
    fixture.statics = PLAINS_ABILITY;
    let fixture = Box::leak(Box::new(fixture));
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&*fixture]))));
    let original = include_str!("../../../scenarios/phased-jihad-triggers-before-a-forest-enters.cantrip");
    let text = original.replace("Oubliette", "Stench Test Oubliette")
        .replace("Assassin's Trophy\n", "Black Lotus\nStench of Evil\n")
        .replace("Bot1.Battlefield.Swamp.Activate\nBot1.Battlefield.Forest.Activate\nBot1.Hand.Assassin's Trophy.Cast(Bot2.Battlefield.Stench Test Oubliette)\nBot2.Accept\nBot2.Search(Forest)",
            "Bot1.Hand.Black Lotus.Cast\nBot1.Battlefield.Black Lotus.Activate(Color: Black)\nBot1.Battlefield.Swamp.Activate\nBot1.Hand.Stench of Evil.Cast\n[Handover Bot2.Decline]");
    let text = text.split("[Expect]").next().unwrap();
    let result = run_on(&GameScript::parse(text, "stench phase boundary").unwrap(), table).map_err(|e| e.report).unwrap();
    assert!(matches!(result.game.pending().map(|r| &r.kind), Some(RequestKind::UnlessPays { .. })));
    assert!(result.game.state.triggers.iter().any(|t| result.game.name(t.source) == "Jihad"),
        "the phased-in state trigger must be queued before the payment question");
}
