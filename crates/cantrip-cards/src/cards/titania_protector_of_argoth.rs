use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// A 5/3 green Elemental creature token.
pub static ELEMENTAL: CardDef = CardDef {
    name: "Elemental Token",
    token_name: Some(&[Subtype::Elemental]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Elemental]),
    power: Some(5),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// "Return target land card from your graveyard to the battlefield", if it is still a legal target.
fn return_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = targets::object(g, item, 0) {
        g.put_onto_battlefield(land, item.controller, false)?;
    }
    Ok(())
}

/// "Whenever a land you control is put into a graveyard from the battlefield" (CR 700.4): judged on the board as it
/// was before it left, so the land's controller is the one it had there.
fn land_you_control_goes_to_graveyard(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Land)
                && q.controller(*old) == q.controller(source) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn create_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELEMENTAL, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&ELEMENTAL];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, return_land, "When Titania, Protector of Argoth enters, return target land card from your graveyard to the battlefield.")
            .targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD]),
        triggered(when(EventMask::ZONE_CHANGE, land_you_control_goes_to_graveyard), create_elemental, "Whenever a land you control is put into a graveyard from the battlefield, create a 5/3 green Elemental creature token."),
    ],
    ..oracle::TITANIA_PROTECTOR_OF_ARGOTH
};
