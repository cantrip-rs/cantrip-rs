use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Artifact spells" (CR 601.3): the card on top of the library is cast as an artifact spell only if it is an artifact card.
fn artifact_spell(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Artifact)
}

// "You may look at the top card of your library any time" (CR 401.2) and "You may cast artifact spells from the top of your
// library" are the play rules. The granted mana ability is a mana ability (CR 605.1a) whose mana can pay only for a spell
// cast from a zone other than the hand (CR 106.6, 601.2h).
pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopSpells(artifact_spell, None)],
    delayed: &[mana_ability(
        TAP,
        add(Mana::U).only_for(ManaRestriction::NOT_SPELLS_FROM_HAND),
        "{T}: Add {U}. Spend this mana only to cast a spell from anywhere other than your hand.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_artifacts,
        statics::grant_delayed_ability,
        "Artifacts you control have \"{T}: Add {U}. Spend this mana only to cast a spell from anywhere other than your hand.\"",
    )],
    ..oracle::MMMENON_THE_RIGHT_HAND
};
