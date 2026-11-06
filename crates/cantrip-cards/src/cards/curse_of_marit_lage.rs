use cantrip_api::*;
use cantrip_mechanics::{lands, targets::filters, triggers};
use cantrip_oracle as oracle;

/// "When this enchantment enters, tap all Islands." (CR 608.2, 701.26a).
fn tap_all_islands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Island))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.tap_together(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tap_all_islands,
            "When this enchantment enters, tap all Islands.",
        ),
    ],
    untap_bans: &[filters::land_type::<{ lands::sub(Subtype::Island) }>],
    ..oracle::CURSE_OF_MARIT_LAGE
};
