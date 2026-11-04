use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "{0}: You may put a basic Forest card from your hand onto the battlefield. Activate only as a sorcery and only once each turn." (CR 608.2).
fn put_basic_forest_card_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    const RULE: select::Selection = select::Selection::of(select::up_to(1), &[select::basic_land_of_type(Subtype::Forest)]);
    let labels = [(Subtype::Plains, "basic Plains"), (Subtype::Island, "basic Island"), (Subtype::Swamp, "basic Swamp"), (Subtype::Mountain, "basic Mountain"), (Subtype::Forest, "basic Forest")];
    let word = g.words(item.object).subtype(Subtype::Forest);
    let kind = labels.iter().find_map(|&(land, name)| (land == word).then_some(name)).unwrap_or("basic land");
    let rule = RULE.called(kind);
    let selected = choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield, hand, rule)?;
    if let Some(&card) = selected.first() {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                Cost::mana(ManaCost::ZERO),
                put_basic_forest_card_hand,
                "{0}: You may put a basic Forest card from your hand onto the battlefield. Activate only as a sorcery and only once each turn.",
            )
            .sorcery_speed()
            .once_each_turn(),
        mana_ability(
            SACRIFICE_THIS,
            add_n(Mana::G, 2),
            "Sacrifice this enchantment: Add {G}{G}.",
        ),
    ],
    ..oracle::GAEAS_TOUCH
};
