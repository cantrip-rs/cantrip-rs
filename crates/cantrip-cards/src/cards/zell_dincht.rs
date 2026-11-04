use cantrip_api::*;
use cantrip_mechanics::lands::{count_you_control, return_a_land};
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "Zell Dincht gets +1/+0 for each land you control" (CR 613.4c, layer 7c): "you" is its controller, the lands counted as the
/// earlier layers made them (a permanent that is a land, whatever else it is), those of the other players not among them.
fn plus_power_per_land_you_control(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let lands = count_you_control(view, source, &|c| c.is(CardType::Land));
    chars.power = chars.power.map(|v| v + lands);
}

pub static CARD: CardDef = CardDef {
    // "You may play an additional land on each of your turns." (CR 305.2: cumulative with other such effects, the ruling.)
    play_rules: &[PlayRule::ExtraLandPlays(1)],
    statics: &[static_ability(Layer::PtModify, itself, plus_power_per_land_you_control, "Zell Dincht gets +1/+0 for each land you control.")],
    // "Return a land you control to its owner's hand" (CR 603.6a): chosen as the ability resolves, not targeted, asked
    // whenever there is a land to choose, and nothing happens with none.
    abilities: &[triggered(YOUR_END_STEP, return_a_land, "At the beginning of your end step, return a land you control to its owner's hand.")],
    ..oracle::ZELL_DINCHT
};
