use cantrip_api::*;
use cantrip_mechanics::combat::{choose_attack_target, create_attacking};
use cantrip_mechanics::sneak;
use cantrip_oracle as oracle;

/// The 1/1 white Ninja Turtle Spirit creature token.
pub static NINJA_TURTLE_SPIRIT: CardDef = CardDef {
    name: "Ninja Turtle Spirit Token",
    token_name: Some(&[Subtype::Ninja, Subtype::Turtle, Subtype::Spirit]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Ninja, Subtype::Turtle, Subtype::Spirit]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "Create three 1/1 white Ninja Turtle Spirit creature tokens. If this spell's sneak cost was paid, they enter tapped and
/// attacking." Sneak paid, their controller chooses what each one attacks as it enters, a player, a planeswalker or a
/// battle (CR 508.4; the ruling: not the thing the returned creature attacked), asked only when there is a choice, and the
/// ones that attack the same thing are created together (CR 101.4). They were never declared as attackers (CR 508.4).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let token = g.card_id(&NINJA_TURTLE_SPIRIT);
    if sneak::paid(g, item) {
        let mut chosen: Vec<Target> = Vec::new();
        for _ in 0..3 {
            chosen.extend(choose_attack_target(g, item.controller, item.object)?);
        }
        create_attacking(g, item.controller, token, true, &chosen)?;
        // With nothing to attack (no defender is left to choose) they still enter tapped, as the sneak cost says.
        g.create_tokens(token, item.controller, 3 - chosen.len() as u32, true)?;
    } else {
        g.create_tokens(token, item.controller, 3, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[sneak::way(mana("{1}{W}"), "Sneak {1}{W}")],
    ..oracle::THE_LAST_RONINS_TECHNIQUE
};

pub static EXTRA: &[&CardDef] = &[&NINJA_TURTLE_SPIRIT];
