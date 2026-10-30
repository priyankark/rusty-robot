use rusty_robot::{Rect, Robot, SearchOptions};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
fn main() {
    let r = Robot;
    for line in io::stdin().lock().lines() {
        let answer=(|| -> Result<Value,Box<dyn std::error::Error>> {
            let v:Value=serde_json::from_str(&line?)?;
            let result=match v["op"].as_str().unwrap_or("") {
                "probe" => json!({"accessibility":r.get_accessibility_permission()?,"capture":r.get_screen_capture_permission()?,"displays":r.get_displays()?.len(),"cursor":{"x":r.get_mouse_pos()?.x,"y":r.get_mouse_pos()?.y}}),
                "click" => {r.move_mouse(v["x"].as_i64().unwrap(),v["y"].as_i64().unwrap())?;r.mouse_click(None,false)?;json!({})},
                "type" => {r.type_string(v["text"].as_str().unwrap())?;json!({})},
                "key" => {r.key_tap(v["key"].as_str().unwrap(),&[])?;json!({})},
                "find" => {
                    let rect=Rect{x:v["x"].as_i64().unwrap(),y:v["y"].as_i64().unwrap(),width:v["width"].as_u64().unwrap() as usize,height:v["height"].as_u64().unwrap() as usize};
                    let image=r.capture(Some(rect))?;
                    let p=image.find_color("00ff00",SearchOptions{tolerance:0.08,..Default::default()})?.ok_or("Target color missing from fresh capture")?;
                    let p=image.to_screen_point(p,None)?;
                    json!({"x":p.x+8,"y":p.y+8,"width":image.width,"height":image.height,"scale":image.scale_x})
                },
                _=>return Err("Unknown command".into())
            };Ok(json!({"ok":true,"result":result}))
        })().unwrap_or_else(|e|json!({"ok":false,"error":e.to_string()}));
        println!("{answer}");
        io::stdout().flush().unwrap();
    }
}
