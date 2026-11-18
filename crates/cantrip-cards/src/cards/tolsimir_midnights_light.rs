use cantrip_api::*;
use cantrip_mechanics::triggers::{you_attack_with_a_creature_of_subtype, THIS_ENTERS};
use cantrip_mechanics::{history, lands, targets, tokens};
use cantrip_oracle as oracle;

/// Voja Fenstalker, the legendary 5/5 green and white Wolf creature token with trample Tolsimir makes (CR 111.1, 205.4a).
pub static VOJA_FENSTALKER: CardDef = CardDef {
    name: "Voja Fenstalker",
    has_cost: false,
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Wolf]),
    power: Some(5),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Trample]),
    text: "Trample",
    ..CardDef::BLANK
};

/// "When Tolsimir enters, create Voja Fenstalker, a legendary 5/5 green and white Wolf creature token with trample."
fn make_voja(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VOJA_FENSTALKER, 1)?;
    Ok(())
}

/// "If Tolsimir attacked this combat" (CR 603.4, 508.1): he was declared as an attacker in this combat phase - he may have left
/// since, and one put onto the battlefield attacking was never declared. Asked as the ability triggers and again as it resolves.
/// The attackers are declared together (CR 508.1k, 603.2c), so as the Wolf's ability triggers the history, which leaves out the
/// batch being judged, does not have Tolsimir's own declaration yet: he is then the attacker the declaration just made. A Wolf
/// can only attack as the declaration is made, so a Tolsimir attacking at that moment was declared with it. Once the ability is
/// on the stack the history has it.
fn tolsimir_attacked_this_combat(q: &dyn Query, asker: &Asker) -> bool {
    q.is_attacking(asker.source) || history::attacked_this_combat(q, asker.source)
}

/// "Target creature an opponent controls blocks that Wolf this combat if able" (CR 509.1c, 611.2a): a requirement on that creature
/// to block the attacking Wolf, the ability's subject, for the rest of this combat; it is obeyed as far as the restrictions
/// allow (a tapped creature, or one that can't block that Wolf, blocks as it may). Nothing if the target is no longer legal
/// (CR 608.2b).
fn must_block_that_wolf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(creature), Some(wolf)) = (targets::object(g, item, 0), item.subject.object()) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::MustBlockAttacker(wolf), Duration::EndOfCombat);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, make_voja, "When Tolsimir enters, create Voja Fenstalker, a legendary 5/5 green and white Wolf creature token with trample."),
        triggered(
            when_asked(EventMask::ATTACKS, you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Wolf) }>),
            must_block_that_wolf,
            "Whenever a Wolf you control attacks, if Tolsimir attacked this combat, target creature an opponent controls blocks that Wolf this combat if able.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])
        .condition(tolsimir_attacked_this_combat),
    ],
    ..oracle::TOLSIMIR_MIDNIGHTS_LIGHT
};

pub static EXTRA: &[&CardDef] = &[&VOJA_FENSTALKER];
