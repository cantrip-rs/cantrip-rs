//! Only blocked attacking creatures are legal choices for Jarkeld (CR 115.1c).
use cantrip_engine::{RequestKind, Target};
use cantrip_script::{run, GameScript};

#[test]
fn refuses_noncreatures_nonattackers_and_unblocked_attackers() {
    let text = include_str!("../../../scenarios/general-jarkeld-target-filter.cantrip");
    let prefix = text.split("! Bot1.Target(Count: 1)").next().unwrap();
    let script = GameScript::parse(&format!("{prefix}[Handover Bot1.Target]\n"), "jarkeld-filter").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let request = result.game.pending().expect("target choice");
    let RequestKind::ChooseTarget { candidates, .. } = &request.kind else { panic!("expected target choice") };
    let mut names: Vec<_> = candidates.iter().filter_map(|t| match t {
        Target::Object(id) => Some(result.game.cards.get(result.game.state.obj(*id).card).name),
        _ => None,
    }).collect();
    names.sort_unstable();
    assert_eq!(names, ["Memnite", "Phyrexian Walker"]);
}
