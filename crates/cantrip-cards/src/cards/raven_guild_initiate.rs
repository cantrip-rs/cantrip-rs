use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

fn bird(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Bird))
}

pub static CARD: CardDef = CardDef { ways: &[morph::way(return_to_hand(1, bird), "Morph\u{2014}Return a Bird you control to its owner's hand.")], ..oracle::RAVEN_GUILD_INITIATE };
