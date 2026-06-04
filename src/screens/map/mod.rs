mod camera;
pub mod map_editor;
mod toolbar;

use camera::Camera;
use toolbar::ToolbarAction;
use toolbar::ToolBar;

fn is_worth_drawing(staring_point: iced::Point, ending_point: iced::Point) -> bool {
    (staring_point.x - ending_point.x).abs() >= 5.0
        && (staring_point.y - ending_point.y).abs() >= 5.0
}
