use std::{collections::HashMap, fmt::Debug};

use plotlib::{page::Page, repr::Plot, style::{LineJoin, LineStyle}, view::ContinuousView};
use rand::{rng, seq::SliceRandom};

use crate::data_type::timewrap::hash_map_to_btree;

use crate::data_type::timewrap::TimeWrap;

#[derive(Clone, Debug)]
pub enum ClusterClass {
    Good(f64, f64),
    Outline(f64, f64),
    Reclusterization(f64, f64),
    NotClassified
}

impl ClusterClass {
    pub fn make_good(&self) -> Self {
        match self {
            Self::Reclusterization(score, sigma) => Self::Good(*score, *sigma),
            Self::Outline(score, sigma) => Self::Good(*score, *sigma),
            Self::Good(score, sigma) => Self::Good(*score, *sigma),
            _ => Self::NotClassified
        }
    }
    pub fn make_outline(&self) -> Self {
        match self {
            Self::Reclusterization(score, sigma) => Self::Outline(*score, *sigma),
            Self::Outline(score, sigma) => Self::Outline(*score, *sigma),
            Self::Good(score, sigma) => Self::Outline(*score, *sigma),
            _ => Self::NotClassified
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClusterSet<T> {
    pub id: usize,
    pub points: HashMap<usize, (f64, f64)>,
    pub centroid: T,
    pub class: ClusterClass,
}

impl ClusterSet<TimeWrap> {
    pub fn new(id: usize, points: &HashMap<usize, (f64, f64)>, centroid: &TimeWrap) -> Self {


        Self { id: id, points: points.clone(), centroid: centroid.clone(), class: ClusterClass::NotClassified }
    }

    pub fn to_svg(&self, data: &HashMap<usize, HashMap<usize, f64>>, path: &String) {
        let colors = vec![
            "#DD3355", // Red-pink
            "#33DD55", // Green
            "#3355DD", // Blue
            "#DDDD33", // Yellow
            "#FF7733", // Orange
            "#AA33DD", // Purple
            "#55AAAA", // Cyan
            "#AAAAAA", // Gray
            "#000000", // Black
            "#DD33DD", // White (not recommended on a white background)
        ];
    
        let mut v = ContinuousView::new();
        let mut points = self.points.keys().copied().collect::<Vec<usize>>().clone();
        points.shuffle(&mut rng());
        for (color_id, point_id) in points.iter().enumerate().take(10) {

            let li = Plot::new(hash_map_to_btree(&data[point_id]).iter().map(|(x, y)| (*x as f64, *y)).collect()).line_style(
                LineStyle::new()
                    .colour(colors[color_id % 10])
                    .linejoin(LineJoin::Round).width(1.0),
            );
            v = v.add(li);

        };
        let l1 = Plot::new(self.centroid.to_btree().iter().map(|(x, y)| (*x as f64, *y)).collect()).line_style(
            LineStyle::new()
                .colour("black")
                .linejoin(LineJoin::Round).width(4.0),
        );
        v = v.add(l1);
        //dbg!(path.clone());
        Page::single(&v).save(path).expect("saving svg");
    }
    pub fn to_csv(&self,) -> String {
        let mut result: String = "".to_string();
        // id, x_centroid, y_centroid, 
        for (x, y) in self.centroid.to_btree().into_iter() {
            result = format!("{}{};{};{}\n", result, self.id, x, y);
        }
        result
    }
    pub fn to_csv_info(&self,) -> String {
        let result: String = format!("{}", match self.class {
            ClusterClass::Good(score, sigma) => format!("{};{};{}", score, sigma, "good"),
            ClusterClass::Reclusterization(score, sigma) => format!("{};{};{}", score, sigma, "reclust"),
            ClusterClass::Outline(score, sigma) =>format!("{};{};{}", score, sigma, "outline"),
            _ => format!(";;not_class"),
        });
        result
    }
}