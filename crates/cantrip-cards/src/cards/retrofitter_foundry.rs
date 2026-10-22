use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::SERVO;
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// A 4/4 colorless Construct artifact creature token.
pub static CONSTRUCT: CardDef = CardDef {
    name: "Construct Token",
    token_name: Some(&[Subtype::Construct]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Construct]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};

fn is_servo(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Servo))
}

fn is_thopter(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Thopter))
}

/// {3}: Untap this artifact.
fn untap_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.untap(item.object);
    }
    Ok(())
}

fn make_servo(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &SERVO, 1)?;
    Ok(())
}

fn make_thopter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &THOPTER, 1)?;
    Ok(())
}

fn make_construct(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &CONSTRUCT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{3}"), untap_this, "{3}: Untap this artifact."),
        ability(cost("{2}, {T}"), make_servo, "{2}, {T}: Create a 1/1 colorless Servo artifact creature token."),
        ability(cost("{1}, {T}").and(sacrifice(1, is_servo)), make_thopter, "{1}, {T}, Sacrifice a Servo: Create a 1/1 colorless Thopter artifact creature token with flying."),
        ability(TAP.and(sacrifice(1, is_thopter)), make_construct, "{T}, Sacrifice a Thopter: Create a 4/4 colorless Construct artifact creature token."),
    ],
    ..oracle::RETROFITTER_FOUNDRY
};

pub static EXTRA: &[&CardDef] = &[&SERVO, &THOPTER, &CONSTRUCT];
