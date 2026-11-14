use cantrip_api::*;
use cantrip_mechanics::banding;
use cantrip_oracle as oracle;

/// "White legendary creatures you control have "bands with other legendary creatures."" (CR 604.1).
fn white_legendary_creatures_control_have_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).colors.has(q.words(asker.source).color(Color::White))
        && (*so_far).supertypes.legendary && (*so_far).is(CardType::Creature)
        && asker.controller == q.controller(affected)
}

/// "White legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)" (CR 702.22c, 702.22h, 702.22j-k).
fn granted_0_bands_0(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).supertypes.legendary && q.chars(id).is(CardType::Creature)
}

/// "White legendary creatures you control have "bands with other legendary creatures."" (CR 604.1).
fn white_legendary_creatures_control_have_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let card = {
        let o = q.state().obj(asker.source);
        o.copied_card().unwrap_or(o.card)
    };
    let _ = card;
    let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) else { return };
    chars.abilities[slot] = Some(
        AbilityRef::delayed(
            {
                let o = q.state().obj(asker.source);
                o.copied_card().unwrap_or(o.card)
            },
            0,
        ),
    );
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            white_legendary_creatures_control_have_affects,
            white_legendary_creatures_control_have_apply_0,
            "White legendary creatures you control have \"bands with other legendary creatures.\"",
        ),
    ],
    delayed: &[banding::with_others(granted_0_bands_0, "BandsWithOthers")],
    ..oracle::CATHEDRAL_OF_SERRA
};
