mod camera;
pub mod map_editor;
mod toolbar;

use camera::Camera;
use toolbar::ToolBar;
use toolbar::ToolbarAction;

use crate::models::Grave;

fn is_worth_drawing(staring_point: iced::Point, ending_point: iced::Point) -> bool {
    (staring_point.x - ending_point.x).abs() >= 5.0
        && (staring_point.y - ending_point.y).abs() >= 5.0
}

fn find_grave_at(graves: &[Grave], point: iced::Point) -> Option<usize> {
     graves.iter().enumerate().find(|&(_, g,)| {
        let min_x = g.coordinate.p1.x.min(g.coordinate.p2.x);
        let max_x = g.coordinate.p1.x.max(g.coordinate.p2.x);

        let min_y = g.coordinate.p1.y.min(g.coordinate.p2.y);
        let max_y = g.coordinate.p1.y.max(g.coordinate.p2.y);

        (min_x..=max_x).contains(&point.x) && (min_y..=max_y).contains(&point.y) 
     }).map(|(idx, _)| idx)
}