//! Catalog spotting matrix, without constructing or running a battle.
//! Run: cargo run -p sim --release --example spotting_report
//! Prints metres and writes throwaway/spotting/report.html; no arguments.
use std::fmt::Write;

use contract::catalog::AltitudeLayer;
use contract::scenario::Rules;
use sim::sensing::{effective, ground, target};

fn group<K: PartialEq>(groups: &mut Vec<(K, Vec<String>)>, key: K, name: String) {
    if let Some((_, names)) = groups.iter_mut().find(|(k, _)| *k == key) {
        names.push(name);
    } else {
        groups.push((key, vec![name]));
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn names(html: &mut String, names: &[String], kind: &str) {
    write!(
        html,
        "<details><summary>{} {kind} units</summary><small>Scroll for all names</small><ul>",
        names.len()
    )
    .unwrap();
    let mut names: Vec<_> = names.iter().collect();
    names.sort_unstable();
    for name in names {
        write!(html, "<li>{}</li>", escape(name)).unwrap();
    }
    html.push_str("</ul></details>");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rules: Rules = serde_json::from_value(sim::fixtures::rules())?;
    let mut targets = Vec::new();
    let mut observers = Vec::new();
    // Only squads garrison, so only their groups get a garrisoned row.
    let mut garrisons = Vec::new();
    for index in rules.catalog.indices() {
        let unit = rules.catalog.get(index);
        let sensors = &unit.sensors;
        let name = format!("{} ({})", unit.name, rules.catalog.id(index));
        group(
            &mut observers,
            (
                sensors.ground_m * sensors.sight_shape.front,
                sensors.spotting,
            ),
            name.clone(),
        );
        if unit.mobility.layer() == AltitudeLayer::Ground {
            group(&mut targets, unit.concealment, name);
            if unit.hull().is_none() {
                garrisons.push(unit.concealment);
            }
        }
    }
    targets.sort_by(|a, b| a.0.total_cmp(&b.0));
    observers.sort_by(|(a, _), (b, _)| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));

    let assumptions = "Front arc; clear line of sight; no foliage along the line; \
        garrisoned at full shelter, for squads only. Airborne targets excluded. Distances in metres.";
    let scope = "Cells isolate authored ground sight and concealment math; altitude sight \
        restrictions are not evaluated.";
    println!("{assumptions}");
    println!("{scope}");
    println!(
        "Forest concealment: {}; building range multiplier: {}; moving concealment: {}.\n",
        rules.forests.rule.concealment,
        rules.sensors.building_range_multiplier,
        rules.sensors.moving_concealment
    );
    let mut html = String::from(
        r#"<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Spotting report</title>
<style>
:root{color-scheme:dark;font-family:system-ui,sans-serif;background:#111820;color:#e6edf3}
body{margin:0;padding:28px}h1{margin:0 0 12px}p{color:#b8c7d5;max-width:100ch;line-height:1.5}
.matrix{overflow:auto;max-height:75vh;border:1px solid #405163;margin-top:22px}
table{border-collapse:separate;border-spacing:0;width:100%;font-variant-numeric:tabular-nums}
th,td{padding:10px 12px;border-right:1px solid #405163;border-bottom:1px solid #405163}
thead th{position:sticky;top:0;background:#1c2936;z-index:2;min-width:105px}
th{text-align:left}thead th:first-child{left:0;z-index:3;min-width:160px}
tbody th[rowspan]{position:sticky;left:0;background:#1c2936;vertical-align:top;z-index:1;min-width:160px}
td{text-align:right;white-space:nowrap;color:#fff}small{display:block;color:#b8c7d5;font-weight:normal}
details{font-size:12px;font-weight:normal;margin-top:8px;max-width:230px}
summary{cursor:pointer;color:#add7ff}ul{padding-left:16px;max-height:230px;overflow:auto}
li{margin:4px 0;overflow-wrap:anywhere}tbody th:not([rowspan]){font-size:13px;font-weight:normal;white-space:nowrap}
.group-start>*{border-top:2px solid #7c96af}
</style>
<h1>Spotting report</h1>"#,
    );
    write!(
        html,
        "<p>{assumptions}<br>{scope}</p><p>Forest concealment: {} · Building range multiplier: {} · \
         Moving concealment: {}<br>Heat shows fraction of full front sight: dark = short, \
         bright teal = full reach. Expand unit lists to inspect group membership.</p>\
         <div class=matrix><table><thead><tr><th scope=col>Target concealment</th>\
         <th scope=col>Ground / motion</th>",
        rules.forests.rule.concealment,
        rules.sensors.building_range_multiplier,
        rules.sensors.moving_concealment
    )?;
    for (i, ((sight, spotting), units)) in observers.iter().enumerate() {
        println!("O{}: front sight {sight} m, spotting {spotting}", i + 1);
        write!(
            html,
            "<th scope=col>O{} · {sight} m<small>Spotting ×{spotting}</small>",
            i + 1
        )?;
        names(&mut html, units, "observer");
        html.push_str("</th>");
    }
    html.push_str("</tr></thead>");
    print!("\nConcealment | Ground     | Motion");
    for i in 0..observers.len() {
        print!(" | {:>7}", format!("O{}", i + 1));
    }
    println!();
    for (concealment, units) in &targets {
        html.push_str("<tbody>");
        let mut grounds = vec![
            ("Open", 1.0, 0.0),
            ("Forest", rules.forests.rule.concealment, 0.0),
        ];
        if garrisons.contains(concealment) {
            grounds.push(("Garrisoned", 1.0, 1.0));
        }
        let rows = 2 * grounds.len();
        for (terrain_index, (terrain, forest, shelter)) in grounds.into_iter().enumerate() {
            for moving in [false, true] {
                let motion = if moving { "moving" } else { "still" };
                let first = terrain_index == 0 && !moving;
                html.push_str(if first {
                    "<tr class=group-start>"
                } else {
                    "<tr>"
                });
                if first {
                    write!(html, "<th rowspan={rows} scope=rowgroup>×{concealment}")?;
                    names(&mut html, units, "target");
                    html.push_str("</th>");
                }
                write!(html, "<th scope=row>{terrain}<small>{motion}</small></th>")?;
                print!("{concealment:>11} | {terrain:<10} | {motion:<6}");
                let net = target(
                    *concealment,
                    ground(forest, shelter, &rules.sensors),
                    moving,
                    &rules.sensors,
                );
                for ((sight, spotting), _) in &observers {
                    let fraction = effective(net, *spotting);
                    let metres = sight * fraction;
                    print!(" | {metres:7.2}");
                    write!(
                        html,
                        "<td style=\"background:hsl(174 55% {:.1}%)\">{metres:.2}</td>",
                        10.0 + 20.0 * fraction
                    )?;
                }
                println!();
                html.push_str("</tr>");
            }
        }
        html.push_str("</tbody>");
    }
    html.push_str("</table></div></html>\n");
    let directory = sim::fixtures::dir().join("../throwaway/spotting");
    std::fs::create_dir_all(&directory)?;
    let output = directory.join("report.html");
    std::fs::write(&output, html)?;
    println!("\nHTML: {}", output.display());
    Ok(())
}
