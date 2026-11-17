use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Activated abilities of artifacts, creatures, and planeswalkers can't be activated." (CR 602.5.) Mana abilities are
/// activated abilities too (CR 605.1a), so they are forbidden as well, and it binds every player, its controller included.
/// A description that names a card type and no zone means a permanent of that type on the battlefield (CR 109.2), so a
/// card in a hand or graveyard keeps its abilities, Elvish Spirit Guide's among them. Null Rod and Stony Silence are the same rule
/// over artifacts alone.
fn may_activate(q: &dyn Query, _: &Asker, _: PlayerId, source: ObjectId, _: &ActivatedDef) -> bool {
    if !q.is_on_battlefield(source) {
        return true;
    }
    let c = q.chars(source);
    !(c.is(CardType::Artifact) || c.is(CardType::Creature) || c.is(CardType::Planeswalker))
}

pub static CARD: CardDef = CardDef { activation_rules: &[may_activate], ..oracle::CLARION_CONQUEROR };
