use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "During your turn, your opponents can't ... activate abilities of artifacts, creatures, or enchantments." (CR 101.2, 602.5.)
/// Judged on the source as it is now, on the battlefield: a card in a hand or graveyard is no artifact, creature or enchantment
/// permanent, so its cycling or channel still works. Mana abilities are barred too (this text has no exception), loyalty abilities
/// are a planeswalker's and special actions are no activated ability (CR 116.2), so both stay allowed. "Your opponents" is the
/// activator, not the source's controller: an ability any player may activate (Armageddon Clock's, CR 602.2) is barred to an
/// opponent on a Clock this player controls, and this player may still activate the one on a Clock an opponent controls.
fn artifact_creature_or_enchantment_abilities_barred(q: &dyn Query, asker: &Asker, activator: PlayerId, source: ObjectId, _: &ActivatedDef) -> bool {
    let chars = q.chars(source);
    !(q.state().turn.active == asker.controller
        && activator != asker.controller
        && q.is_on_battlefield(source)
        && (chars.is(CardType::Artifact) || chars.is(CardType::Creature) || chars.is(CardType::Enchantment)))
}

pub static CARD: CardDef = CardDef {
    cast_rules: &[statics::opponents_cant_cast_spells_during_your_turn],
    activation_rules: &[artifact_creature_or_enchantment_abilities_barred],
    ..oracle::GRAND_ABOLISHER
};

pub static EXTRA: &[&CardDef] = &[];
