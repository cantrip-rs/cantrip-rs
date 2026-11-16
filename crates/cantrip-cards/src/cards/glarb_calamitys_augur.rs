use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Spells with mana value 4 or greater" (CR 601.3, 202.3): the card on top as the spell it would be.
fn mana_value_four_or_greater(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.mana_value(card) >= 4
}

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2); "you may play lands and cast spells with mana value 4
    // or greater from the top of your library" (CR 305.1, 601.3): normal timing, normal costs.
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands, PlayRule::LibraryTopSpells(mana_value_four_or_greater, None)],
    abilities: &[ability(TAP, |g, item| effects::surveil(g, item, 2), "{T}: Surveil 2.")],
    ..oracle::GLARB_CALAMITYS_AUGUR
};
