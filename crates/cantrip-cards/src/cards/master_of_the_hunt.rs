use cantrip_api::*;
use cantrip_mechanics::banding;
use cantrip_oracle as oracle;

/// "{2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has "bands with other creatures named Wolves of the Hunt." (Any creatures named Wolves of the Hunt can attack in a band as long as at least one has "bands with other creatures named Wolves of the Hunt." Bands are blocked as a group. If at least two creatures named Wolves of the Hunt you control, one of which has "bands with other creatures named Wolves of the Hunt," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)" (CR 702.22b).
fn create_1_1_green_wolf_bands_0(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).name == "Wolves of the Hunt"
}

pub static WOLVES_OF_THE_HUNT: CardDef = CardDef {
    name: "Wolves of the Hunt",
    text: "Bands with other creatures named Wolves of the Hunt.",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wolf]),
    colors: Some(ColorSet(16)),
    power: Some(1),
    toughness: Some(1),
    abilities: &[banding::with_others(create_1_1_green_wolf_bands_0, "Bands with other creatures named Wolves of the Hunt.")],
    ..CardDef::BLANK
};

/// "{2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has "bands with other creatures named Wolves of the Hunt."" (CR 608.2).
fn create_1_1_green_wolf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&WOLVES_OF_THE_HUNT);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{G}{G}"),
            create_1_1_green_wolf,
            "{2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has \"bands with other creatures named Wolves of the Hunt.\"",
        ),
    ],
    ..oracle::MASTER_OF_THE_HUNT
};

pub static EXTRA: &[&CardDef] = &[&WOLVES_OF_THE_HUNT];
