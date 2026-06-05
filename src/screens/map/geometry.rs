use crate::models::Grave;

pub fn is_worth_drawing(starting_point: iced::Point, ending_point: iced::Point) -> bool {
    (starting_point.x - ending_point.x).abs() >= 5.0
        && (starting_point.y - ending_point.y).abs() >= 5.0
}

pub fn find_grave_at(graves: &[Grave], point: iced::Point) -> Option<usize> {
    graves.iter().position(|grave| grave.contains(point))
}
