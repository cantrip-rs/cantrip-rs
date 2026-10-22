use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// The 5/5 green Wurm creature token with trample it makes.
pub static WURM: CardDef = CardDef {
    name: "Wurm Token",
    token_name: Some(&[Subtype::Wurm]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wurm]),
    power: Some(5),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Trample]),
    ..CardDef::BLANK
};

/// "When this creature dies, create three 5/5 green Wurm creature tokens with trample."
fn three_wurms(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WURM, 3)?;
    Ok(())
}

/// "When this card is put into a graveyard from anywhere" (CR 603.6c, 603.10a): its own move into a graveyard, from any
/// zone. It functions from the graveyard it has just reached.
fn put_into_a_graveyard(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Graveyard) => Fired::PLAIN,
        _ => None,
    }
}

/// "Shuffle it into its owner's library": the card that triggered it goes there if it is still where it was put (CR 400.7), and
/// the library is shuffled either way (CR 701.24c); the core does both.
fn shuffle_it_in(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_into_library(&[item.object], LibraryPlace::ShuffledIn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_DIES, three_wurms, "When this creature dies, create three 5/5 green Wurm creature tokens with trample."),
        triggered(when(EventMask::ZONE_CHANGE, put_into_a_graveyard), shuffle_it_in, "When Worldspine Wurm is put into a graveyard from anywhere, shuffle it into its owner's library.")
            .anywhere(),
    ],
    ..oracle::WORLDSPINE_WURM
};

pub static EXTRA: &[&CardDef] = &[&WURM];
