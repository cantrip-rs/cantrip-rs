use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

fn zombie_card(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Zombie))
}

pub static CARD: CardDef = CardDef { ways: &[morph::way(discard(1, zombie_card), "Morph\u{2014}Discard a Zombie card.")], ..oracle::PUTRID_RAPTOR };
