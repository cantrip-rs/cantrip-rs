//! Illusionary Mask in the two cases no implemented card reaches yet: a creature card with {S} in its mana cost (Icehide
//! Golem prints one), and a creature the Mask cast, turned face up, then turned face down again in place (Backslide,
//! Ixidron).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// A creature card whose mana cost is {S} (CR 107.4h): one mana from a snow source pays it, no other.
static SNOW_PROBE: CardDef = CardDef {
    name: "Probe Snow Golem",
    oracle: true,
    cost: ManaCost { snow: 1, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: false, snow: true, world: false },
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SNOW_PROBE])))
}

/// "Whose mana cost could be paid by ... the mana you spent on {X}": {U} from Mox Sapphire, no snow source, can't pay {S};
/// {U} from a Snow-Covered Island can (CR 107.4h), each activation reading its own payment's mana.
#[test]
fn only_mana_from_a_snow_source_spent_on_x_pays_a_snow_symbol() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Illusionary Mask
Mox Pearl
Mox Jet
Mox Sapphire
Snow-Covered Island
Probe Snow Golem
Island x20
[Bot2 Deck]
Island x24
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Illusionary Mask.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Illusionary Mask.Activate(X: 1)
Bot1.Pass
! Bot1.Choose(Probe Snow Golem)
Bot1.Choose()
Bot1.Pass
Bot1.Hand.Snow-Covered Island.Play
Bot1.Battlefield.Snow-Covered Island.Activate
Bot1.Battlefield.Illusionary Mask.Activate(X: 1)
Bot1.Pass
Bot1.Choose(Probe Snow Golem)
Bot1.Accept
Bot1.Pass
Bot2.Pass
[Bot1.Main2]
[Expect]
Bot1.Battlefield..FaceDownName = Probe Snow Golem
"#, "illusionary-mask-snow").unwrap();
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}", failure.report);
    }
}

/// "If the creature that spell becomes ... has not been turned face up": Caelorna, Coral Tyrant (0/8), cast face down by the
/// Mask, is turned face up as its attack taps it. Turned face down again where it stands (as Backslide would, CR 708.8 -
/// written into the finished game's state here, since no implemented card does it and the engine has no instruction for
/// it), it has been turned face up, and none of the Mask's three effects applies to it any more: not to damage dealt to it,
/// to its becoming tapped, nor to its assigning combat damage. Face down and never turned up, each does.
#[test]
fn a_creature_turned_up_and_face_down_again_is_no_longer_the_masks() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Illusionary Mask
Mox Pearl
Mox Jet
Mox Sapphire
Mox Sapphire
Caelorna, Coral Tyrant
Island x20
[Bot2 Deck]
Island x24
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Illusionary Mask.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Mox Sapphire#1.Activate
Bot1.Battlefield.Mox Sapphire#2.Activate
Bot1.Battlefield.Illusionary Mask.Activate(X: 2)
Bot1.Pass
Bot1.Choose(Caelorna, Coral Tyrant)
Bot1.Accept
Bot1.Pass
Bot2.Pass
[T3 Bot1.DeclareAttackers]
Bot1.Attack(#1 > Bot2)
[T3 Bot1.Main2]
[Expect]
Bot1.Battlefield.Caelorna, Coral Tyrant.Tapped = true
"#, "illusionary-mask-face-down-again").unwrap();
    let mut game = run_on(&script, table()).map_err(|f| f.report).unwrap().game;
    let mask_def = cantrip_cards::all().into_iter().find(|d| d.name == "Illusionary Mask").expect("the Mask");
    let mask = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Illusionary Mask").expect("the Mask in play");
    let creature = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Caelorna, Coral Tyrant").expect("Caelorna, face up");
    let asker = Asker { instance: None, source: mask, controller: PlayerId(0), by: None, spell: false };
    let events = [
        Proposal::Damage(DamageEvent { source: mask, target: Target::Object(creature), amount: 3, combat: false, heal_marked: false, shielded: None }),
        Proposal::Tap(Tap { permanent: creature, controller: PlayerId(0) }),
        Proposal::CombatDamageAssignment(CombatDamageAssignment { creature, controller: PlayerId(0) }),
    ];
    let applies = |game: &Game| -> Vec<bool> {
        mask_def.granted_replacements.iter().zip(&events).map(|(def, event)| (def.applies)(game, &asker, creature, event)).collect()
    };
    // Face up: none applies.
    assert_eq!(applies(&game), [false; 3]);
    let morphed = game.cards.id_of(&cantrip_api::intrinsic::MORPHED);
    game.state.update_obj(creature, |o| {
        o.face_down = true;
        o.marks.face_down_as = Some(morphed);
    });
    // Face down again, but turned face up since it entered: none applies still.
    assert_eq!(applies(&game), [false; 3]);
    // A face-down creature never turned up is one each applies to: the same object, with its turning up gone from the record.
    let records = game.log.len();
    game.log.retain(|r| !matches!(r.event, Event::TurnedFaceUp { permanent, .. } if permanent == creature));
    assert!(game.log.len() < records);
    assert_eq!(applies(&game), [true; 3]);
}
