use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// I, II: "Destroy up to one target nonland permanent" (CR 701.8); a target that has become illegal is left alone
/// (CR 608.2b), and choosing none does nothing.
fn destroy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.target_count > 0 {
        destroy_target(g, item, 0);
    }
    Ok(())
}

/// IV: "This creature deals damage equal to the total mana value of other permanents you control to each opponent."
/// The total is worked out as the chapter resolves (CR 608.2h, the ruling), over every permanent the chapter's
/// controller has but the Saga itself, as the game reads mana value (a token's is 0, an {X} permanent's X is 0, a copy
/// has the copied card's); each opponent is dealt it at once (CR 120.3).
fn mega_flare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let total: u32 = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object).map(|id| g.mana_value(id)).sum();
    if total > 0 {
        let hits: Vec<(ObjectId, Target, i64)> =
            g.state().opponents(item.controller).map(|opponent| (item.object, Target::Player(opponent), i64::from(total.min(u16::MAX as u32) as u16))).collect();
        g.damage_together(&hits);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, destroy_it, "I \u{2014} Destroy up to one target nonland permanent.").targets(&[up_to_one(targets::NONLAND_PERMANENT.filter)]),
        chapter(2, destroy_it, "II \u{2014} Destroy up to one target nonland permanent.").targets(&[up_to_one(targets::NONLAND_PERMANENT.filter)]),
        chapter(3, |g, item| effects::draw(g, item, 2), "III \u{2014} Draw two cards."),
        chapter(4, mega_flare, "IV \u{2014} Mega Flare \u{2014} This creature deals damage equal to the total mana value of other permanents you control to each opponent."),
    ],
    ..oracle::SUMMON_BAHAMUT
};
