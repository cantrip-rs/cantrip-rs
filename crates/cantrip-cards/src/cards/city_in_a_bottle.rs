use cantrip_api::*;
use cantrip_oracle as oracle;

/// "A name originally printed in the Arabian Nights expansion": every card name whose first printing is Arabian Nights
/// (Scryfall `set:arn not:reprint`; Mountain, the set's one reprint, was first printed in Alpha), whatever the printing
/// of the card that has it now (the 2014-02-01 ruling). City in a Bottle's own name is one of them.
fn originally_printed_in_arabian_nights(name: &str) -> bool {
    matches!(
        name,
        "Abu Ja'far" | "Aladdin" | "Aladdin's Lamp" | "Aladdin's Ring" | "Ali Baba" | "Ali from Cairo" | "Army of Allah"
            | "Bazaar of Baghdad" | "Bird Maiden" | "Bottle of Suleiman" | "Brass Man" | "Camel" | "City in a Bottle"
            | "City of Brass" | "Cuombajj Witches" | "Cyclone" | "Dancing Scimitar" | "Dandân" | "Desert"
            | "Desert Nomads" | "Desert Twister" | "Diamond Valley" | "Drop of Honey" | "Ebony Horse"
            | "Elephant Graveyard" | "El-Hajjâj" | "Erg Raiders" | "Erhnam Djinn" | "Eye for an Eye" | "Fishliver Oil"
            | "Flying Carpet" | "Flying Men" | "Ghazbán Ogre" | "Giant Tortoise" | "Guardian Beast" | "Hasran Ogress"
            | "Hurr Jackal" | "Ifh-Bíff Efreet" | "Island Fish Jasconius" | "Island of Wak-Wak" | "Jandor's Ring"
            | "Jandor's Saddlebags" | "Jeweled Bird" | "Jihad" | "Junún Efreet" | "Juzám Djinn" | "Khabál Ghoul"
            | "King Suleiman" | "Kird Ape" | "Library of Alexandria" | "Magnetic Mountain" | "Merchant Ship"
            | "Metamorphosis" | "Mijae Djinn" | "Moorish Cavalry" | "Nafs Asp" | "Oasis" | "Old Man of the Sea"
            | "Oubliette" | "Piety" | "Pyramids" | "Repentant Blacksmith" | "Ring of Ma'rûf" | "Rukh Egg"
            | "Sandals of Abdallah" | "Sandstorm" | "Serendib Djinn" | "Serendib Efreet" | "Shahrazad" | "Sindbad"
            | "Singing Tree" | "Sorceress Queen" | "Stone-Throwing Devils" | "Unstable Mutation" | "War Elephant"
            | "Wyluli Wolf" | "Ydwen Efreet"
    )
}

/// A nontoken permanent with such a name, by its name now (a copy effect's, CR 707.2; a face-down permanent has none, CR
/// 708.2). A token is never one, whatever it copies (the 2004-10-04 ruling).
fn nontoken_arabian_nights_permanent(q: &dyn Query, id: ObjectId) -> bool {
    !q.state().obj(id).token && originally_printed_in_arabian_nights(q.chars(id).name)
}

/// "Whenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on
/// the battlefield": a state trigger (CR 603.8), true while any permanent other than this one is such a permanent. It
/// triggers once, and again only once it has left the stack (the 2014-02-01 ruling: checked whenever a player would
/// receive priority).
fn other_arabian_nights_permanents(q: &dyn Query, asker: &Asker) -> bool {
    q.state().battlefield().iter().any(|&id| id != asker.source && nontoken_arabian_nights_permanent(q, id))
}

/// "Them": the permanents that were such permanents as it triggered (`TriggeredDef::members_of`), this one among them if it
/// is one - it is left out as the ability resolves, a state trigger being about no object of its own to tell it by here.
fn those_permanents(q: &dyn Query, _: Target) -> Vec<ObjectId> {
    q.state().battlefield().iter().copied().filter(|&id| nontoken_arabian_nights_permanent(q, id)).collect()
}

/// "... their controllers sacrifice them" (CR 701.21a, 101.4): each of those permanents other than this one, by its
/// controller as the ability resolves, all at once. One that has left the battlefield since is a new object and stays
/// where it is (CR 400.7); one that entered after the ability triggered is not one of them, and is sacrificed by the
/// ability's next triggering once this one has left the stack (CR 603.8).
fn controllers_sacrifice_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let them: Vec<ObjectId> = g.event_objects(item).iter().copied().filter(|&id| id != item.object).collect();
    let picks: Vec<(PlayerId, ObjectId)> = them.into_iter().map(|id| (g.controller(id), id)).collect();
    g.sacrifice_each(&picks);
    Ok(())
}

/// "Players can't cast spells ... with a name originally printed in the Arabian Nights expansion" (CR 101.2, 601.3): any
/// player, the spell judged by its name as it would be cast.
fn not_arabian_nights_spells(q: &dyn Query, _: &Asker, _: PlayerId, spell: ObjectId) -> bool {
    !originally_printed_in_arabian_nights(q.chars(spell).name)
}

/// "Players can't ... play lands with a name originally printed in the Arabian Nights expansion" (CR 101.2, 305.1): any
/// player, from anywhere, the land judged by the face it would be played as. A land put onto the battlefield is not played.
fn not_arabian_nights_lands(q: &dyn Query, _: &Asker, _: PlayerId, _: ObjectId, face: CardId) -> bool {
    !originally_printed_in_arabian_nights(q.card(face).name)
}

pub static CARD: CardDef = CardDef {
    abilities: &[state_triggered(
        other_arabian_nights_permanents,
        controllers_sacrifice_them,
        "Whenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on the battlefield, their controllers sacrifice them.",
    )
    .members_of(those_permanents)],
    spell_rules: &[not_arabian_nights_spells],
    land_play_rules: &[not_arabian_nights_lands],
    ..oracle::CITY_IN_A_BOTTLE
};
